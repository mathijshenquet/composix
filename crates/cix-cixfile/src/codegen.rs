use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

use crate::parser::bare_command;
use crate::{
    Assembly, BuildStep, Cixfile, InputLock, Item, LockFile, Port, Service, Template, TemplatePart,
};

pub fn generate_spec_json(cixfile: &Cixfile) -> Result<String> {
    let (_, item) = only_item(cixfile)?;
    let paths = item_paths(cixfile, item);
    let value = literal_spec(item, &paths)?;
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
    let (name, _) = only_item(cixfile)?;
    generate_nix_with_snapshot(cixfile, name, source_dir, lock, system, None)
}

pub(crate) fn generate_nix_with_snapshot(
    cixfile: &Cixfile,
    item_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    build_snapshot: Option<&str>,
) -> Result<String> {
    lock.validate_for(&cixfile.inputs)?;
    let item = cixfile
        .items
        .get(item_name)
        .with_context(|| format!("unknown Cixfile ITEM {item_name:?}"))?;
    let paths = item_paths(cixfile, item);
    if build_snapshot.is_none() && !item.takes.is_empty() {
        bail!("TAKE requires cix build to execute the COPY/FETCH/RUN chain");
    }
    if let Some(snapshot) = build_snapshot {
        if !snapshot.starts_with("/nix/store/") {
            bail!("build snapshot is not a Nix store path: {snapshot}");
        }
    }
    source_dir
        .canonicalize()
        .with_context(|| format!("resolving Cixfile directory {}", source_dir.display()))?;

    let mut expression = String::new();
    writeln!(expression, "let")?;
    for (index, (name, _)) in cixfile.inputs.iter().enumerate() {
        let input = &lock.inputs[name];
        writeln!(expression, "  input{index}Source = {};", fetch_tree(input)?)?;
    }
    writeln!(expression, "  universes = {{")?;
    for (index, (name, _)) in cixfile.inputs.iter().enumerate() {
        writeln!(
            expression,
            "    {} = import input{index}Source {{ system = {}; }};",
            nix_attr(name),
            nix_string(system)
        )?;
    }
    writeln!(expression, "  }};")?;
    if let Some(snapshot) = build_snapshot {
        writeln!(
            expression,
            "  buildSnapshot = builtins.storePath {};",
            nix_string(snapshot)
        )?;
    }
    writeln!(
        expression,
        "  pathDirs = {};",
        nix_templates(&paths, build_snapshot)
    )?;
    if !paths.is_empty() {
        writeln!(expression, "  resolveExecutable = line: command:")?;
        writeln!(expression, "    let")?;
        writeln!(
            expression,
            "      candidates = map (directory: \"${{directory}}/${{command}}\") pathDirs;"
        )?;
        writeln!(
            expression,
            "      found = builtins.filter (candidate: builtins.substring 0 1 candidate != \"/\" || builtins.pathExists candidate) candidates;"
        )?;
        writeln!(expression, "    in if found == [] then")?;
        writeln!(
            expression,
            r#"      throw "line ${{builtins.toString line}}: command ${{command}} was not found in declared PATH directories: ${{builtins.concatStringsSep ", " pathDirs}}""#
        )?;
        writeln!(expression, "    else builtins.head found;")?;
    }
    writeln!(
        expression,
        "  spec = {};",
        nix_spec(item, &paths, build_snapshot)
    )?;

    for (index, assembly) in item.assembly.iter().enumerate() {
        match assembly {
            Assembly::File { contents, .. } => {
                writeln!(
                    expression,
                    "  file{index} = universes.{}.writeText \"cixfile-file-{index}\" {};",
                    nix_attr(primary_namespace(cixfile)?),
                    nix_template(contents, build_snapshot)
                )?;
            }
            Assembly::Script { contents, .. } => {
                writeln!(
                    expression,
                    "  script{index} = universes.{}.writeText \"cixfile-script-{index}\" (\"#!${{universes.{}.runtimeShell}}\\n\" + {});",
                    nix_attr(primary_namespace(cixfile)?),
                    nix_attr(primary_namespace(cixfile)?),
                    nix_template(contents, build_snapshot)
                )?;
            }
            Assembly::Link { target, .. } => {
                writeln!(
                    expression,
                    "  link{index} = {};",
                    nix_template(target, build_snapshot)
                )?;
            }
        }
    }
    for (index, take) in item.takes.iter().enumerate() {
        writeln!(
            expression,
            "  take{index} = {};",
            nix_take_source(&take.src, build_snapshot)
        )?;
    }
    writeln!(
        expression,
        "  manifestFile = universes.{}.writeText \"cix-manifest.json\" (builtins.toJSON spec + \"\\n\");",
        nix_attr(primary_namespace(cixfile)?)
    )?;
    writeln!(expression, "in")?;
    writeln!(
        expression,
        "universes.{}.runCommand {} {{ preferLocalBuild = true; allowSubstitutes = false; }} ''",
        nix_attr(primary_namespace(cixfile)?),
        nix_string(&format!("cix-item-{item_name}"))
    )?;
    writeln!(expression, "  set -eu")?;
    writeln!(expression, "  mkdir -p \"$out\"")?;
    for directory in item_directories(item) {
        writeln!(
            expression,
            "  mkdir -p \"$out/{}\"",
            shell_double_quoted(&directory)
        )?;
    }
    for (index, assembly) in item.assembly.iter().enumerate() {
        match assembly {
            Assembly::File { dst, .. } => writeln!(
                expression,
                "  install -m 0644 ${{file{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Assembly::Script { dst, .. } => writeln!(
                expression,
                "  install -m 0755 ${{script{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Assembly::Link { dst, .. } => writeln!(
                expression,
                "  ln -s ${{universes.{}.lib.escapeShellArg link{index}}} \"$out/{}\"",
                nix_attr(primary_namespace(cixfile)?),
                shell_double_quoted(dst)
            )?,
        }
    }
    for (index, take) in item.takes.iter().enumerate() {
        writeln!(
            expression,
            "  if [ ! -e \"${{take{index}}}\" ] && [ ! -L \"${{take{index}}}\" ]; then echo {} >&2; exit 1; fi",
            nix_string(&format!(
                "line {}: TAKE source does not exist",
                take.line
            ))
        )?;
        writeln!(
            expression,
            "  cp -a \"${{take{index}}}\" \"$out/{}\"",
            shell_double_quoted(&take.dst)
        )?;
    }
    {
        let service = &item.service;
        for (index, (arguments, line)) in [
            (&service.exec[..], service.exec_line),
            (
                service.setup.as_deref().unwrap_or_default(),
                service.setup_line.unwrap_or_default(),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let Some(command) = bare_command(arguments) else {
                continue;
            };
            writeln!(
                expression,
                "  resolved{index}=${{universes.{}.lib.escapeShellArg (resolveExecutable {line} {})}}",
                nix_attr(primary_namespace(cixfile)?),
                nix_string(&command),
            )?;
            writeln!(
                expression,
                "  case \"$resolved{index}\" in /*) executable{index}=\"$resolved{index}\" ;; *) executable{index}=\"$out/$resolved{index}\" ;; esac"
            )?;
            writeln!(
                expression,
                "  test -x \"$executable{index}\" || {{ echo {} >&2; exit 1; }}",
                nix_string(&format!(
                    "line {line}: resolved PATH command {command:?} is not executable"
                )),
            )?;
        }
    }
    writeln!(
        expression,
        "  install -m 0644 ${{manifestFile}} \"$out/cix-manifest.json\""
    )?;
    writeln!(expression, "''")?;
    Ok(expression)
}

pub(crate) fn generate_build_context_nix(
    cixfile: &Cixfile,
    lock: &LockFile,
    system: &str,
) -> Result<String> {
    lock.validate_for(&cixfile.inputs)?;
    let mut expression = nix_prelude(cixfile, lock, system)?;
    let package_refs = package_references(cixfile);
    writeln!(expression, "in {{")?;
    writeln!(
        expression,
        "  offers = [ {} ];",
        package_refs
            .iter()
            .map(|(namespace, attrpath)| {
                format!(
                    "(builtins.toString {})",
                    package_expression(namespace, attrpath)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    let build_paths = cixfile
        .paths
        .iter()
        .filter(|path| {
            !path
                .parts
                .iter()
                .any(|part| matches!(part, TemplatePart::Build { .. }))
        })
        .cloned()
        .collect::<Vec<_>>();
    writeln!(
        expression,
        "  paths = {};",
        nix_templates(&build_paths, None)
    )?;
    let commands = cixfile
        .steps
        .iter()
        .filter_map(|step| match step {
            BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } => Some(command),
            BuildStep::Copy { .. } => None,
        })
        .cloned()
        .collect::<Vec<_>>();
    writeln!(
        expression,
        "  commands = {};",
        nix_templates(&commands, None)
    )?;
    writeln!(expression, "  environment = {{")?;
    for (name, template) in build_environment(cixfile)? {
        writeln!(
            expression,
            "    {} = {};",
            nix_attr(&name),
            nix_template(&template, None)
        )?;
    }
    writeln!(expression, "  }};")?;
    writeln!(expression, "}}")?;
    Ok(expression)
}

pub(crate) fn generate_offer_build_nix(
    cixfile: &Cixfile,
    lock: &LockFile,
    system: &str,
) -> Result<String> {
    lock.validate_for(&cixfile.inputs)?;
    let mut expression = nix_prelude(cixfile, lock, system)?;
    writeln!(
        expression,
        "in [ {} ]",
        package_references(cixfile)
            .iter()
            .map(|(namespace, attrpath)| package_expression(namespace, attrpath))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    Ok(expression)
}

fn nix_prelude(cixfile: &Cixfile, lock: &LockFile, system: &str) -> Result<String> {
    let mut expression = String::new();
    writeln!(expression, "let")?;
    for (index, (name, _)) in cixfile.inputs.iter().enumerate() {
        let input = &lock.inputs[name];
        writeln!(expression, "  input{index}Source = {};", fetch_tree(input)?)?;
    }
    writeln!(expression, "  universes = {{")?;
    for (index, (name, _)) in cixfile.inputs.iter().enumerate() {
        writeln!(
            expression,
            "    {} = import input{index}Source {{ system = {}; }};",
            nix_attr(name),
            nix_string(system)
        )?;
    }
    writeln!(expression, "  }};")?;
    Ok(expression)
}

fn package_expression(namespace: &str, attrpath: &str) -> String {
    let mut expression = format!("universes.{}", nix_attr(namespace));
    for component in attrpath.split('.') {
        expression.push('.');
        expression.push_str(&nix_attr(component));
    }
    expression
}

fn package_references(cixfile: &Cixfile) -> BTreeSet<(String, String)> {
    fn add(template: &Template, refs: &mut BTreeSet<(String, String)>) {
        for part in &template.parts {
            if let TemplatePart::Package {
                namespace,
                attrpath,
                ..
            } = part
            {
                refs.insert((namespace.clone(), attrpath.clone()));
            }
        }
    }

    let mut refs = BTreeSet::new();
    for path in &cixfile.paths {
        add(path, &mut refs);
    }
    for step in &cixfile.steps {
        if let BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } = step {
            add(command, &mut refs);
        }
    }
    for item in cixfile.items.values() {
        for path in &item.paths {
            add(path, &mut refs);
        }
        for assembly in &item.assembly {
            match assembly {
                Assembly::File { contents, .. } | Assembly::Script { contents, .. } => {
                    add(contents, &mut refs);
                }
                Assembly::Link { target, .. } => add(target, &mut refs),
            }
        }
        let service = &item.service;
        for argument in &service.exec {
            add(argument, &mut refs);
        }
        if let Some(arguments) = &service.setup {
            for argument in arguments {
                add(argument, &mut refs);
            }
        }
        for default in service
            .env
            .values()
            .filter_map(|environment| environment.default.as_ref())
        {
            add(default, &mut refs);
        }
    }
    refs
}

fn build_environment(cixfile: &Cixfile) -> Result<std::collections::BTreeMap<String, Template>> {
    let mut environment: std::collections::BTreeMap<String, Template> =
        std::collections::BTreeMap::new();
    for item in cixfile.items.values() {
        let service = &item.service;
        for (name, declaration) in &service.env {
            let Some(default) = &declaration.default else {
                continue;
            };
            if default
                .parts
                .iter()
                .any(|part| matches!(part, TemplatePart::Build { .. }))
            {
                continue;
            }
            if let Some(existing) = environment.get(name) {
                if !existing.same_value(default) {
                    bail!("build environment {name:?} has conflicting defaults across ITEM blocks");
                }
            } else {
                environment.insert(name.clone(), default.clone());
            }
        }
    }
    Ok(environment)
}

fn item_directories(item: &Item) -> BTreeSet<String> {
    item.assembly
        .iter()
        .map(|assembly| match assembly {
            Assembly::File { dst, .. }
            | Assembly::Script { dst, .. }
            | Assembly::Link { dst, .. } => dst,
        })
        .chain(item.takes.iter().map(|take| &take.dst))
        .filter_map(|destination| {
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

fn fetch_tree(input: &InputLock) -> Result<String> {
    if input.url.starts_with("github:") {
        let (owner, repo) = github_repository(&input.url)?;
        return Ok(format!(
            "builtins.fetchTree {{ type = \"github\"; owner = {}; repo = {}; rev = {}; narHash = {}; }}",
            nix_string(owner),
            nix_string(repo),
            nix_string(&input.rev),
            nix_string(&input.nar_hash),
        ));
    }
    if input.url.starts_with("https://") {
        return Ok(format!(
            "builtins.fetchTree {{ type = \"tarball\"; url = {}; narHash = {}; }}",
            nix_string(&input.url),
            nix_string(&input.nar_hash),
        ));
    }
    bail!("unsupported FROM URL {:?}", input.url)
}

fn primary_namespace(cixfile: &Cixfile) -> Result<&str> {
    cixfile
        .inputs
        .keys()
        .next()
        .map(String::as_str)
        .context("Cixfile has no FROM input")
}

fn only_item(cixfile: &Cixfile) -> Result<(&str, &Item)> {
    if cixfile.items.len() != 1 {
        bail!(
            "this operation requires exactly one ITEM, found {}",
            cixfile.items.len()
        );
    }
    let (name, item) = cixfile.items.first_key_value().expect("one item");
    Ok((name, item))
}

fn item_paths(cixfile: &Cixfile, item: &Item) -> Vec<Template> {
    cixfile.paths.iter().chain(&item.paths).cloned().collect()
}

fn nix_spec(item: &Item, paths: &[Template], build_snapshot: Option<&str>) -> String {
    let mounts = projected_mounts(item);
    let mut output = String::from("{ cixManifest = 4;");
    let service = nix_service(&item.service, &mounts, paths, build_snapshot);
    output.push_str(service.trim_start_matches('{').trim_end_matches('}'));
    output.push_str(" }");
    output
}

fn nix_service(
    service: &Service,
    mounts: &BTreeSet<String>,
    paths: &[Template],
    build_snapshot: Option<&str>,
) -> String {
    let mut output = String::from("{");
    write!(
        output,
        " exec = {};",
        nix_command(&service.exec, service.exec_line, build_snapshot)
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
            nix_command(
                setup,
                service.setup_line.expect("SETUP has a line"),
                build_snapshot,
            )
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
                write!(
                    output,
                    " default = {};",
                    nix_template(default, build_snapshot)
                )
                .unwrap();
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
    if service.outbound {
        output.push_str(" outbound = true;");
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

fn nix_templates(templates: &[Template], build_snapshot: Option<&str>) -> String {
    format!(
        "[ {} ]",
        templates
            .iter()
            .map(|template| nix_template(template, build_snapshot))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn nix_command(arguments: &[Template], line: usize, build_snapshot: Option<&str>) -> String {
    let Some(command) = bare_command(arguments) else {
        return nix_templates(arguments, build_snapshot);
    };
    let mut output = format!("[ (resolveExecutable {line} {})", nix_string(&command));
    for argument in &arguments[1..] {
        write!(output, " {}", nix_template(argument, build_snapshot)).unwrap();
    }
    output.push_str(" ]");
    output
}

fn nix_template(template: &Template, build_snapshot: Option<&str>) -> String {
    let mut output = String::from("\"");
    for part in &template.parts {
        match part {
            TemplatePart::Literal(value) => output.push_str(&escape_nix_string(value)),
            TemplatePart::Package {
                namespace,
                attrpath,
                line,
            } => {
                output.push_str("${");
                output.push_str("builtins.addErrorContext ");
                output.push_str(&nix_string(&format!(
                    "Cixfile line {line}: resolving {namespace}.{attrpath}"
                )));
                output.push(' ');
                output.push_str("universes.");
                output.push_str(&nix_attr(namespace));
                for component in attrpath.split('.') {
                    output.push('.');
                    output.push_str(&nix_attr(component));
                }
                output.push('}');
            }
            TemplatePart::Build { .. } => {
                build_snapshot.expect("${build} was validated before codegen");
                output.push_str("${buildSnapshot}");
            }
        }
    }
    output.push('"');
    output
}

fn nix_take_source(template: &Template, build_snapshot: Option<&str>) -> String {
    match template.parts.as_slice() {
        [TemplatePart::Literal(path)] => {
            build_snapshot.expect("TAKE was validated before codegen");
            format!("\"${{buildSnapshot}}/{}\"", escape_nix_string(path))
        }
        _ => nix_template(template, build_snapshot),
    }
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

fn literal_spec(item: &Item, paths: &[Template]) -> Result<Value> {
    let mounts = projected_mounts(item);
    let Value::Object(mut value) = literal_service(&item.service, &mounts, paths)? else {
        unreachable!("service literal is an object");
    };
    value.insert("cixManifest".to_owned(), Value::from(4));
    Ok(Value::Object(value))
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
    if service.outbound {
        value.insert("outbound".into(), Value::Bool(true));
    }
    Ok(Value::Object(value))
}

fn projected_mounts(item: &Item) -> BTreeSet<String> {
    item.assembly
        .iter()
        .map(|assembly| match assembly {
            Assembly::File { dst, .. }
            | Assembly::Script { dst, .. }
            | Assembly::Link { dst, .. } => dst,
        })
        .chain(item.takes.iter().map(|take| &take.dst))
        .filter_map(|destination| {
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

    fn fixture_lock() -> LockFile {
        LockFile {
            inputs: std::collections::BTreeMap::from([(
                "pkgs".into(),
                crate::InputLock {
                    url: "github:NixOS/nixpkgs/nixos-unstable".into(),
                    rev: "0123456789abcdef".into(),
                    nar_hash: "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
                },
            )]),
            fetches: std::collections::BTreeMap::new(),
            memo: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn golden_cixfile_generates_expected_spec() {
        let cixfile = parse(include_str!("../tests/golden/Cixfile")).unwrap();
        let actual = generate_spec_json(&cixfile).unwrap();
        assert_eq!(actual, include_str!("../tests/golden/cix-manifest.json"));
    }

    #[test]
    fn groups_projected_destinations_without_broadening_mounts() {
        let cixfile = parse(
            "FROM nixpkgs AS pkgs\nITEM app\nFILE /etc/nginx/nginx.conf <<E\nevents {}\nE\nLINK /etc/nginx/mime.types /mime.types\nFILE /srv/www/index.html <<E\nhello\nE\nFILE /cix-probe.conf <<E\nprobe\nE\nEXEC bin/app\n",
        )
        .unwrap();
        let spec = generate_spec_json(&cixfile).unwrap();
        assert!(spec.contains("\"mounts\": [\n    \"/cix-probe.conf\",\n    \"/etc/nginx\",\n    \"/srv/www\"\n  ]"), "{spec}");

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
    fn listener_emits_the_v4_bare_contract() {
        let cixfile =
            parse("FROM nixpkgs AS pkgs\nITEM web\nEXEC bin/web\nLISTENER http\nLISTENER admin\n")
                .unwrap();
        let spec = generate_spec_json(&cixfile).unwrap();
        assert!(spec.contains("\"cixManifest\": 4"), "{spec}");
        assert!(!spec.contains("\"services\""), "{spec}");
        assert!(
            spec.contains(
                "\"listeners\": {\n    \"admin\": {\n      \"type\": \"stream\"\n    },\n    \"http\": {\n      \"type\": \"stream\"\n    }\n  }"
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
        assert!(nix.contains("cixManifest = 4;"), "{nix}");
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
        let cixfile = parse(
            "FROM nixpkgs AS pkgs\nITEM app\nLINK bin/hello ${pkgs.hello}/bin/hello\nEXEC bin/hello\n",
        )
        .unwrap();
        let first =
            generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux").unwrap();
        let second =
            generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux").unwrap();
        assert_eq!(first, second);
        assert!(first.contains("builtins.fetchTree"));
        assert!(first.contains("narHash = \"sha256-"));
        assert!(first.contains("universes.\"pkgs\".\"hello\""));
        assert!(first.contains("Cixfile line 3: resolving pkgs.hello"));
    }

    #[test]
    fn rejects_non_github_lock_urls() {
        let directory = tempfile::tempdir().unwrap();
        let cixfile = parse("FROM nixpkgs AS pkgs\nITEM app\nEXEC /bin/x\n").unwrap();
        let lock = LockFile {
            inputs: std::collections::BTreeMap::new(),
            fetches: std::collections::BTreeMap::new(),
            memo: std::collections::BTreeMap::new(),
        };
        let error = generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux")
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing FROM input"), "{error}");
    }
}
