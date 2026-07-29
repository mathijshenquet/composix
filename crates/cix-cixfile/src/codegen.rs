use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

use crate::parser::bare_command;
use crate::{Cixfile, Item, LockFile, Port, Service, Template, TemplatePart};

pub fn generate_spec_json(cixfile: &Cixfile) -> Result<String> {
    let value = literal_spec(cixfile)?;
    let mut json = serde_json::to_string_pretty(&value)?;
    json.push('\n');
    Ok(json)
}

pub fn generate_nix(
    cixfile: &Cixfile,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
) -> Result<String> {
    let (owner, repo) = github_repository(&lock.nixpkgs.url)?;
    let source_dir = source_dir
        .canonicalize()
        .with_context(|| format!("resolving Cixfile directory {}", source_dir.display()))?;

    let mut expression = String::new();
    writeln!(expression, "let")?;
    writeln!(expression, "  nixpkgsSource = builtins.fetchTree {{")?;
    writeln!(expression, "    type = \"github\";")?;
    writeln!(expression, "    owner = {};", nix_string(owner))?;
    writeln!(expression, "    repo = {};", nix_string(repo))?;
    writeln!(expression, "    rev = {};", nix_string(&lock.nixpkgs.rev))?;
    writeln!(
        expression,
        "    narHash = {};",
        nix_string(&lock.nixpkgs.nar_hash)
    )?;
    writeln!(expression, "  }};")?;
    writeln!(
        expression,
        "  pkgs = import nixpkgsSource {{ system = {}; }};",
        nix_string(system)
    )?;
    writeln!(
        expression,
        "  pathDirs = {};",
        nix_templates(&cixfile.paths)
    )?;
    if !cixfile.paths.is_empty() {
        writeln!(expression, "  resolveExecutable = line: command:")?;
        writeln!(expression, "    let")?;
        writeln!(
            expression,
            "      candidates = map (directory: \"${{directory}}/${{command}}\") pathDirs;"
        )?;
        writeln!(
            expression,
            "      found = builtins.filter builtins.pathExists candidates;"
        )?;
        writeln!(expression, "    in if found == [] then")?;
        writeln!(
            expression,
            r#"      throw "line ${{builtins.toString line}}: command ${{command}} was not found in declared PATH directories: ${{builtins.concatStringsSep ", " pathDirs}}""#
        )?;
        writeln!(expression, "    else builtins.head found;")?;
    }
    writeln!(expression, "  spec = {};", nix_spec(cixfile))?;

    for (index, item) in cixfile.items.iter().enumerate() {
        match item {
            Item::Copy { src, .. } => {
                let path = source_dir.join(src);
                let metadata = fs::metadata(&path)
                    .with_context(|| format!("COPY source {} does not exist", path.display()))?;
                if !metadata.is_file() {
                    bail!("COPY source {} is not a regular file", path.display());
                }
                writeln!(
                    expression,
                    "  copy{index} = builtins.path {{ path = builtins.toPath {}; name = \"cixfile-copy-{index}\"; }};",
                    nix_string(path.to_str().context("COPY source path is not valid UTF-8")?)
                )?;
            }
            Item::File { contents, .. } => {
                writeln!(
                    expression,
                    "  file{index} = pkgs.writeText \"cixfile-file-{index}\" {};",
                    nix_template(contents)
                )?;
            }
            Item::Script { contents, .. } => {
                writeln!(
                    expression,
                    "  script{index} = pkgs.writeText \"cixfile-script-{index}\" (\"#!${{pkgs.runtimeShell}}\\n\" + {});",
                    nix_template(contents)
                )?;
            }
            Item::Link { target, .. } => {
                writeln!(expression, "  link{index} = {};", nix_template(target))?;
            }
        }
    }
    writeln!(
        expression,
        "  specFile = pkgs.writeText \"cix-spec.json\" (builtins.toJSON spec + \"\\n\");"
    )?;
    writeln!(expression, "in")?;
    writeln!(
        expression,
        "pkgs.runCommand \"cixfile-item\" {{ preferLocalBuild = true; allowSubstitutes = false; }} ''"
    )?;
    writeln!(expression, "  set -eu")?;
    writeln!(expression, "  mkdir -p \"$out\"")?;

    for service in cixfile.services.values() {
        for (arguments, line) in [
            (&service.exec[..], service.exec_line),
            (
                service.setup.as_deref().unwrap_or_default(),
                service.setup_line.unwrap_or_default(),
            ),
        ] {
            let Some(command) = bare_command(arguments) else {
                continue;
            };
            writeln!(
                expression,
                "  test -x ${{pkgs.lib.escapeShellArg (resolveExecutable {line} {})}} || {{ echo {} >&2; exit 1; }}",
                nix_string(&command),
                nix_string(&format!("line {line}: resolved PATH command {command:?} is not executable")),
            )?;
        }
    }

    for directory in item_directories(cixfile) {
        writeln!(
            expression,
            "  mkdir -p \"$out/{}\"",
            shell_double_quoted(&directory)
        )?;
    }
    for (index, item) in cixfile.items.iter().enumerate() {
        match item {
            Item::Copy { dst, .. } => writeln!(
                expression,
                "  install -m 0644 ${{copy{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Item::File { dst, .. } => writeln!(
                expression,
                "  install -m 0644 ${{file{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Item::Script { dst, .. } => writeln!(
                expression,
                "  install -m 0755 ${{script{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Item::Link { dst, .. } => writeln!(
                expression,
                "  ln -s ${{pkgs.lib.escapeShellArg link{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
        }
    }
    writeln!(
        expression,
        "  install -m 0644 ${{specFile}} \"$out/cix-spec.json\""
    )?;
    writeln!(expression, "''")?;
    Ok(expression)
}

fn item_directories(cixfile: &Cixfile) -> BTreeSet<String> {
    cixfile
        .items
        .iter()
        .filter_map(|item| {
            let destination = match item {
                Item::Copy { dst, .. }
                | Item::File { dst, .. }
                | Item::Script { dst, .. }
                | Item::Link { dst, .. } => dst,
            };
            Path::new(destination)
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| parent.to_string_lossy().into_owned())
        })
        .collect()
}

fn github_repository(url: &str) -> Result<(&str, &str)> {
    let path = url
        .strip_prefix("github:")
        .with_context(|| format!("unsupported nixpkgs URL {url:?}; expected github:owner/repo"))?;
    let mut components = path.split('/');
    let owner = components.next().filter(|value| !value.is_empty());
    let repo = components.next().filter(|value| !value.is_empty());
    match (owner, repo) {
        (Some(owner), Some(repo)) => Ok((owner, repo)),
        _ => bail!("invalid github nixpkgs URL {url:?}"),
    }
}

fn nix_spec(cixfile: &Cixfile) -> String {
    let version = if cixfile
        .services
        .values()
        .any(|service| !service.listeners.is_empty())
    {
        3
    } else {
        2
    };
    let mut output = format!("{{ cixSpec = {version}; services = {{");
    let mounts = projected_mounts(cixfile);
    for (name, service) in &cixfile.services {
        write!(
            output,
            " {} = {};",
            nix_attr(name),
            nix_service(service, &mounts, &cixfile.paths)
        )
        .unwrap();
    }
    output.push_str(" }; }");
    output
}

fn nix_service(service: &Service, mounts: &BTreeSet<String>, paths: &[Template]) -> String {
    let mut output = String::from("{");
    write!(
        output,
        " exec = {};",
        nix_command(&service.exec, service.exec_line)
    )
    .unwrap();
    if !mounts.is_empty() {
        write!(
            output,
            " mounts = [ {} ];",
            mounts
                .iter()
                .map(|mount| nix_string(mount))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .unwrap();
    }
    if let Some(setup) = &service.setup {
        write!(
            output,
            " setup = {};",
            nix_command(setup, service.setup_line.expect("SETUP has a line"))
        )
        .unwrap();
    }
    if !service.env.is_empty() || !paths.is_empty() {
        output.push_str(" env = {");
        if !paths.is_empty() {
            output.push_str(" \"PATH\" = { default = builtins.concatStringsSep \":\" pathDirs; };");
        }
        for (name, env) in &service.env {
            write!(output, " {} = {{", nix_attr(name)).unwrap();
            if let Some(default) = &env.default {
                write!(output, " default = {};", nix_template(default)).unwrap();
            }
            if env.required {
                output.push_str(" required = true;");
            }
            if env.secret {
                output.push_str(" secret = true;");
            }
            output.push_str(" };");
        }
        output.push_str(" };");
    }
    if !service.ports.is_empty() {
        output.push_str(" ports = {");
        for (name, port) in &service.ports {
            write!(output, " {} = {{", nix_attr(name)).unwrap();
            match port {
                Port::Env(variable) => {
                    write!(output, " env = {};", nix_string(variable)).unwrap();
                }
                Port::Value(value) => {
                    write!(output, " value = {value};").unwrap();
                }
            }
            output.push_str(" protocol = \"tcp\"; };");
        }
        output.push_str(" };");
    }
    if !service.listeners.is_empty() {
        output.push_str(" listeners = {");
        for name in &service.listeners {
            write!(output, " {} = {{ type = \"stream\"; }};", nix_attr(name)).unwrap();
        }
        output.push_str(" };");
    }
    let dirs = nix_dirs(service);
    if let Some(dirs) = dirs {
        write!(output, " dirs = {dirs};").unwrap();
    }
    if service.jit {
        output.push_str(" jit = true;");
    }
    output.push_str(" }");
    output
}

fn nix_dirs(service: &Service) -> Option<String> {
    let roles = [
        ("state", &service.dirs.state),
        ("cache", &service.dirs.cache),
        ("logs", &service.dirs.logs),
        ("config", &service.dirs.config),
        ("run", &service.dirs.run),
    ];
    if roles.iter().all(|(_, paths)| paths.is_empty()) {
        return None;
    }
    let mut output = String::from("{");
    for (role, paths) in roles {
        if !paths.is_empty() {
            write!(
                output,
                " {role} = [ {} ];",
                paths
                    .iter()
                    .map(|path| nix_string(path))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .unwrap();
        }
    }
    output.push_str(" }");
    Some(output)
}

fn nix_templates(templates: &[Template]) -> String {
    format!(
        "[ {} ]",
        templates
            .iter()
            .map(nix_template)
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn nix_command(arguments: &[Template], line: usize) -> String {
    let Some(command) = bare_command(arguments) else {
        return nix_templates(arguments);
    };
    let mut output = format!("[ (resolveExecutable {line} {})", nix_string(&command));
    for argument in &arguments[1..] {
        write!(output, " {}", nix_template(argument)).unwrap();
    }
    output.push_str(" ]");
    output
}

fn nix_template(template: &Template) -> String {
    let mut output = String::from("\"");
    for part in &template.parts {
        match part {
            TemplatePart::Literal(value) => output.push_str(&escape_nix_string(value)),
            TemplatePart::Package(package) => {
                output.push_str("${");
                output.push_str("pkgs");
                for component in package.split('.') {
                    output.push('.');
                    output.push_str(&nix_attr(component));
                }
                output.push('}');
            }
        }
    }
    output.push('"');
    output
}

fn nix_string(value: &str) -> String {
    format!("\"{}\"", escape_nix_string(value))
}

fn nix_attr(value: &str) -> String {
    nix_string(value)
}

fn escape_nix_string(value: &str) -> String {
    let mut output = String::new();
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '$' if characters.peek() == Some(&'{') => output.push_str("\\$"),
            character => output.push(character),
        }
    }
    output
}

fn shell_double_quoted(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

fn literal_spec(cixfile: &Cixfile) -> Result<Value> {
    let mut services = Map::new();
    let mounts = projected_mounts(cixfile);
    for (name, service) in &cixfile.services {
        services.insert(
            name.clone(),
            literal_service(service, &mounts, &cixfile.paths)?,
        );
    }
    let version = if cixfile
        .services
        .values()
        .any(|service| !service.listeners.is_empty())
    {
        3
    } else {
        2
    };
    Ok(Value::Object(Map::from_iter([
        ("cixSpec".to_owned(), Value::from(version)),
        ("services".to_owned(), Value::Object(services)),
    ])))
}

fn literal_service(
    service: &Service,
    mounts: &BTreeSet<String>,
    paths: &[Template],
) -> Result<Value> {
    let mut value = Map::new();
    value.insert("exec".into(), literal_command(&service.exec, paths)?);
    if !mounts.is_empty() {
        value.insert(
            "mounts".into(),
            Value::Array(mounts.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(setup) = &service.setup {
        value.insert("setup".into(), literal_command(setup, paths)?);
    }
    if !service.env.is_empty() || !paths.is_empty() {
        let mut envs = Map::new();
        if !paths.is_empty() {
            envs.insert(
                "PATH".into(),
                Value::Object(Map::from_iter([(
                    "default".into(),
                    Value::String(
                        paths
                            .iter()
                            .map(literal_template)
                            .collect::<Result<Vec<_>>>()?
                            .join(":"),
                    ),
                )])),
            );
        }
        for (name, env) in &service.env {
            let mut declaration = Map::new();
            if let Some(default) = &env.default {
                declaration.insert("default".into(), Value::String(literal_template(default)?));
            }
            if env.required {
                declaration.insert("required".into(), Value::Bool(true));
            }
            if env.secret {
                declaration.insert("secret".into(), Value::Bool(true));
            }
            envs.insert(name.clone(), Value::Object(declaration));
        }
        value.insert("env".into(), Value::Object(envs));
    }
    if !service.ports.is_empty() {
        let mut ports = Map::new();
        for (name, port) in &service.ports {
            let mut declaration = Map::new();
            match port {
                Port::Env(variable) => {
                    declaration.insert("env".into(), Value::String(variable.clone()));
                }
                Port::Value(port) => {
                    declaration.insert("value".into(), Value::from(*port));
                }
            }
            declaration.insert("protocol".into(), Value::String("tcp".into()));
            ports.insert(name.clone(), Value::Object(declaration));
        }
        value.insert("ports".into(), Value::Object(ports));
    }
    if !service.listeners.is_empty() {
        value.insert(
            "listeners".into(),
            Value::Object(
                service
                    .listeners
                    .iter()
                    .map(|name| {
                        (
                            name.clone(),
                            Value::Object(Map::from_iter([(
                                "type".into(),
                                Value::String("stream".into()),
                            )])),
                        )
                    })
                    .collect(),
            ),
        );
    }
    let dirs = literal_dirs(service);
    if !dirs.is_empty() {
        value.insert("dirs".into(), Value::Object(dirs));
    }
    if service.jit {
        value.insert("jit".into(), Value::Bool(true));
    }
    Ok(Value::Object(value))
}

fn projected_mounts(cixfile: &Cixfile) -> BTreeSet<String> {
    cixfile
        .items
        .iter()
        .filter_map(|item| {
            let destination = match item {
                Item::Copy { dst, .. }
                | Item::File { dst, .. }
                | Item::Script { dst, .. }
                | Item::Link { dst, .. } => dst,
            };
            let path = Path::new(destination);
            if !path.is_absolute() {
                return None;
            }
            let components = path
                .components()
                .filter_map(|component| match component {
                    Component::Normal(component) => component.to_str(),
                    _ => None,
                })
                .collect::<Vec<_>>();
            match components.as_slice() {
                [first] => Some(format!("/{first}")),
                [first, second, ..] => Some(format!("/{first}/{second}")),
                [] => None,
            }
        })
        .collect()
}

fn literal_dirs(service: &Service) -> Map<String, Value> {
    let mut dirs = Map::new();
    for (role, paths) in [
        ("state", &service.dirs.state),
        ("cache", &service.dirs.cache),
        ("logs", &service.dirs.logs),
        ("config", &service.dirs.config),
        ("run", &service.dirs.run),
    ] {
        if !paths.is_empty() {
            dirs.insert(
                role.into(),
                Value::Array(paths.iter().cloned().map(Value::String).collect()),
            );
        }
    }
    dirs
}

fn literal_command(arguments: &[Template], paths: &[Template]) -> Result<Value> {
    let mut values = arguments
        .iter()
        .map(literal_template)
        .collect::<Result<Vec<_>>>()?;
    if let Some(command) = bare_command(arguments) {
        let directory = paths
            .first()
            .context("cannot render a bare command without PATH")?;
        values[0] = format!(
            "{}/{command}",
            literal_template(directory)?.trim_end_matches('/')
        );
    }
    Ok(Value::Array(
        values.into_iter().map(Value::String).collect(),
    ))
}

fn literal_template(template: &Template) -> Result<String> {
    template.literal_value().with_context(|| {
        "cannot render package interpolation as JSON without evaluating the generated Nix"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    const LOCK: LockFile = LockFile {
        nixpkgs: crate::NixpkgsLock {
            url: String::new(),
            rev: String::new(),
            nar_hash: String::new(),
        },
    };

    fn fixture_lock() -> LockFile {
        LockFile {
            nixpkgs: crate::NixpkgsLock {
                url: "github:NixOS/nixpkgs/nixos-unstable".into(),
                rev: "0123456789abcdef".into(),
                nar_hash: "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
            },
        }
    }

    #[test]
    fn golden_cixfile_generates_expected_spec() {
        let cixfile = parse(include_str!("../tests/golden/Cixfile")).unwrap();
        let actual = generate_spec_json(&cixfile).unwrap();
        assert_eq!(actual, include_str!("../tests/golden/cix-spec.json"));
    }

    #[test]
    fn groups_projected_destinations_without_broadening_mounts() {
        let cixfile = parse(
            "FILE /etc/nginx/nginx.conf <<E\nevents {}\nE\nLINK /etc/nginx/mime.types /mime.types\nFILE /srv/www/index.html <<E\nhello\nE\nFILE /cix-probe.conf <<E\nprobe\nE\nSERVICE app\nEXEC bin/app\n",
        )
        .unwrap();
        let spec = generate_spec_json(&cixfile).unwrap();
        assert!(spec.contains("\"mounts\": [\n        \"/cix-probe.conf\",\n        \"/etc/nginx\",\n        \"/srv/www\"\n      ]"), "{spec}");

        let nix = generate_nix(
            &cixfile,
            tempfile::tempdir().unwrap().path(),
            &fixture_lock(),
            "x86_64-linux",
        )
        .unwrap();
        assert!(nix.contains("mounts = [ \"/cix-probe.conf\" \"/etc/nginx\" \"/srv/www\" ];"));
    }

    #[test]
    fn listener_emits_the_v3_stream_contract() {
        let cixfile = parse("SERVICE web\nEXEC bin/web\nLISTENER http\nLISTENER admin\n").unwrap();
        let spec = generate_spec_json(&cixfile).unwrap();
        assert!(spec.contains("\"cixSpec\": 3"), "{spec}");
        assert!(
            spec.contains(
                "\"listeners\": {\n        \"admin\": {\n          \"type\": \"stream\"\n        },\n        \"http\": {\n          \"type\": \"stream\"\n        }\n      }"
            ),
            "{spec}"
        );
        let nix = generate_nix(
            &cixfile,
            tempfile::tempdir().unwrap().path(),
            &fixture_lock(),
            "x86_64-linux",
        )
        .unwrap();
        assert!(nix.contains("cixSpec = 3;"), "{nix}");
        assert!(
            nix.contains(
                "listeners = { \"admin\" = { type = \"stream\"; }; \"http\" = { type = \"stream\"; }; };"
            ),
            "{nix}"
        );
    }

    #[test]
    fn nix_generation_is_deterministic_and_uses_fixed_fetch() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("asset"), "hello").unwrap();
        let cixfile = parse(
            "PKG hello\nCOPY asset share/asset\nLINK bin/hello ${hello}/bin/hello\nSERVICE app\nEXEC bin/hello\n",
        )
        .unwrap();
        let first =
            generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux").unwrap();
        let second =
            generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux").unwrap();
        assert_eq!(first, second);
        assert!(first.contains("builtins.fetchTree"));
        assert!(first.contains("narHash = \"sha256-"));
        assert!(first.contains("${pkgs.\"hello\"}/bin/hello"));
        assert!(first.contains("builtins.path"));
    }

    #[test]
    fn copy_must_name_an_existing_regular_file() {
        let directory = tempfile::tempdir().unwrap();
        let cixfile = parse("COPY missing x\nSERVICE app\nEXEC /bin/x\n").unwrap();
        let error = generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux")
            .unwrap_err()
            .to_string();
        assert!(error.contains("does not exist"), "{error}");
    }

    #[test]
    fn rejects_non_github_lock_urls() {
        let directory = tempfile::tempdir().unwrap();
        let cixfile = parse("SERVICE app\nEXEC /bin/x\n").unwrap();
        let error = generate_nix(&cixfile, directory.path(), &LOCK, "x86_64-linux")
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported nixpkgs URL"), "{error}");
    }
}
