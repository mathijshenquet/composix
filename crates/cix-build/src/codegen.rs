use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

use crate::{
    Artifact, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, CopyMode, InputKind, InputLock,
    LockFile, Port, Probe, Service, Template, TemplatePart,
};

fn bare_command(arguments: &[Template]) -> Option<String> {
    let Template { parts } = arguments.first()?;
    match parts.as_slice() {
        [TemplatePart::Literal(command)] if !command.contains('/') => Some(command.clone()),
        _ => None,
    }
}

pub fn generate_spec_json(cixfile: &Cixfile) -> Result<String> {
    let (_, artifact) = only_artifact(cixfile)?;
    if !artifact.kind.is_runnable() {
        bail!("ITEM blocks are content-only and do not have runtime manifests; see docs/cixfile.md#item");
    }
    if !artifact.imports.is_empty() {
        bail!("an artifact with IMPORT needs Nix evaluation to enumerate its sparse mounts");
    }
    let value = literal_spec(artifact)?;
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
    let (name, _) = only_artifact(cixfile)?;
    generate_nix_with_snapshots(cixfile, name, source_dir, lock, system, &BTreeMap::new())
}

pub fn generate_nix_with_snapshots(
    cixfile: &Cixfile,
    artifact_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    lock.validate_for(&cixfile.inputs)?;
    validate_snapshots(snapshots)?;
    let artifact = cixfile
        .artifacts
        .get(artifact_name)
        .with_context(|| format!("unknown Cixfile artifact {artifact_name:?}"))?;
    let source_dir = source_dir
        .canonicalize()
        .with_context(|| format!("resolving Cixfile directory {}", source_dir.display()))?;
    let primary = primary_namespace(cixfile)?;

    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    if artifact.kind.is_runnable() {
        writeln!(expression, "  spec = {};", nix_spec(artifact)?)?;
    }
    for (index, import) in artifact.imports.iter().enumerate() {
        writeln!(expression, "  import{index} = {};", nix_template(import))?;
    }
    if !artifact.imports.is_empty() {
        writeln!(
            expression,
            "  artifactImports = [ {} ];",
            (0..artifact.imports.len())
                .map(|index| format!("import{index}"))
                .collect::<Vec<_>>()
                .join(" ")
        )?;
        writeln!(expression, "  importMounts = builtins.concatMap (package: builtins.concatMap (subtree: let directory = builtins.toString package + \"/\" + subtree; in if builtins.pathExists directory then map (name: \"/\" + subtree + \"/\" + name) (builtins.attrNames (builtins.readDir directory)) else []) [ \"bin\" \"etc\" \"share\" ]) artifactImports;")?;
    }
    for (index, copy) in artifact.copies.iter().enumerate() {
        writeln!(
            expression,
            "  copy{index} = {};",
            nix_artifact_copy_source(copy)
        )?;
    }
    for (index, assembly) in artifact.assembly.iter().enumerate() {
        match assembly {
            Assembly::File { contents, .. } => {
                writeln!(
                    expression,
                    "  file{index} = universes.{}.writeText \"cixfile-file-{index}\" {};",
                    nix_attr(primary),
                    nix_template(contents)
                )?;
            }
            Assembly::Link { target, .. } => {
                writeln!(expression, "  link{index} = {};", nix_template(target))?;
            }
        }
    }
    if artifact.kind.is_runnable() {
        writeln!(
            expression,
            "  manifestFile = universes.{}.writeText \"cix-manifest.json\" (builtins.toJSON spec + \"\\n\");",
            nix_attr(primary)
        )?;
    }
    writeln!(expression, "in")?;
    writeln!(
        expression,
        "universes.{}.runCommand {} {{ preferLocalBuild = true; allowSubstitutes = false; }} ''",
        nix_attr(primary),
        nix_string(&format!("cix-item-{artifact_name}"))
    )?;
    writeln!(expression, "  set -eu")?;
    writeln!(expression, "  mkdir -p \"$out\"")?;
    if !artifact.imports.is_empty() {
        writeln!(expression, "  merge_import() {{")?;
        writeln!(
            expression,
            "    local source=\"$1\" destination=\"$2\" entry target"
        )?;
        writeln!(expression, "    mkdir -p \"$destination\"")?;
        writeln!(
            expression,
            "    for entry in \"$source\"/* \"$source\"/.[!.]* \"$source\"/..?*; do"
        )?;
        writeln!(
            expression,
            "      if [ ! -e \"$entry\" ] && [ ! -L \"$entry\" ]; then continue; fi"
        )?;
        writeln!(expression, "      target=\"$destination/''${{entry##*/}}\"")?;
        writeln!(
            expression,
            "      if [ -d \"$entry\" ] && [ ! -L \"$entry\" ]; then"
        )?;
        writeln!(
            expression,
            "        if [ ! -e \"$target\" ] && [ ! -L \"$target\" ]; then mkdir \"$target\"; fi"
        )?;
        writeln!(expression, "        if [ -d \"$target\" ] && [ ! -L \"$target\" ]; then merge_import \"$entry\" \"$target\"; fi")?;
        writeln!(
            expression,
            "      elif [ ! -e \"$target\" ] && [ ! -L \"$target\" ]; then"
        )?;
        writeln!(expression, "        ln -s \"$entry\" \"$target\"")?;
        writeln!(expression, "      fi")?;
        writeln!(expression, "    done")?;
        writeln!(expression, "  }}")?;
        for index in 0..artifact.imports.len() {
            for subtree in ["bin", "etc", "share"] {
                writeln!(expression, "  if [ -d \"${{import{index}}}/{subtree}\" ]; then merge_import \"${{import{index}}}/{subtree}\" \"$out/{subtree}\"; fi")?;
            }
        }
    }
    for directory in artifact_directories(artifact) {
        writeln!(
            expression,
            "  mkdir -p \"$out/{}\"",
            shell_double_quoted(&directory)
        )?;
    }
    for (index, copy) in artifact.copies.iter().enumerate() {
        emit_copy(&mut expression, index, copy)?;
    }
    for (index, assembly) in artifact.assembly.iter().enumerate() {
        match assembly {
            Assembly::File { dst, .. } => writeln!(
                expression,
                "  install -m 0644 ${{file{index}}} \"$out/{}\"",
                shell_double_quoted(dst)
            )?,
            Assembly::Link { dst, .. } => writeln!(
                expression,
                "  ln -s ${{universes.{}.lib.escapeShellArg link{index}}} \"$out/{}\"",
                nix_attr(primary),
                shell_double_quoted(dst)
            )?,
        }
    }
    if artifact.kind.is_runnable() {
        let service = &artifact.service;
        for (arguments, line, directive) in [
            (&service.start[..], service.start_line, "START"),
            (
                service.start_pre.as_deref().unwrap_or_default(),
                service.start_pre_line.unwrap_or_default(),
                "START_PRE",
            ),
        ]
        .into_iter()
        {
            let Some(command) = bare_command(arguments) else {
                continue;
            };
            writeln!(
                expression,
                "  if ! test -x \"$out/bin/{}\"; then",
                shell_double_quoted(&command),
            )?;
            writeln!(
                expression,
                "    entries=\"\"; if test -d \"$out/bin\"; then entries=\"$(ls -1A \"$out/bin\" | paste -sd ', ' -)\"; fi"
            )?;
            writeln!(
                expression,
                "    if test -n \"$entries\"; then echo \"{} (contains: $entries)\" >&2; else echo \"{} (contains: <empty>)\" >&2; fi; exit 1",
                shell_double_quoted(&format!(
                    "line {line}: bare {directive} command {command:?} was not found in this item's bin/"
                )),
                shell_double_quoted(&format!(
                    "line {line}: bare {directive} command {command:?} was not found in this item's bin/"
                )),
            )?;
            writeln!(expression, "  fi")?;
        }
        writeln!(
            expression,
            "  install -m 0644 ${{manifestFile}} \"$out/cix-manifest.json\""
        )?;
    }
    writeln!(expression, "''")?;
    Ok(expression)
}

pub fn generate_builder_context_nix(
    cixfile: &Cixfile,
    builder_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    let builder = cixfile
        .builders
        .get(builder_name)
        .with_context(|| format!("unknown BUILDER {builder_name:?}"))?;
    let source_dir = source_dir.canonicalize()?;
    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    let templates = builder_command_templates(builder);
    let package_refs = package_references(templates.iter().copied());
    writeln!(expression, "in {{")?;
    writeln!(
        expression,
        "  offers = [ {} ];",
        offer_expressions(&package_refs, templates.iter().copied())
    )?;
    writeln!(
        expression,
        "  imports = {};",
        nix_templates(&builder.imports)
    )?;
    writeln!(
        expression,
        "  commands = {};",
        nix_templates(
            &builder
                .steps
                .iter()
                .filter_map(|step| match step {
                    BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } => {
                        Some(command.clone())
                    }
                    BuildStep::Env { .. } | BuildStep::Copy(_) => None,
                })
                .collect::<Vec<_>>()
        )
    )?;
    writeln!(
        expression,
        "  copies = [ {} ];",
        builder
            .steps
            .iter()
            .filter_map(|step| match step {
                BuildStep::Copy(copy) => Some(nix_copy_source(&copy.src)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    writeln!(expression, "  environment = {{}};")?;
    writeln!(
        expression,
        "  universeIdentities = {};",
        universe_identities(cixfile, lock)
    )?;
    writeln!(expression, "}}")?;
    Ok(expression)
}

pub fn generate_builder_offer_nix(
    cixfile: &Cixfile,
    builder_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    let builder = cixfile
        .builders
        .get(builder_name)
        .with_context(|| format!("unknown BUILDER {builder_name:?}"))?;
    let source_dir = source_dir.canonicalize()?;
    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    let templates = builder_templates(builder);
    let package_refs = package_references(templates.iter().copied());
    writeln!(
        expression,
        "in [ {} ]",
        package_refs
            .iter()
            .map(|(namespace, attrpath)| package_expression(namespace, attrpath))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    Ok(expression)
}

pub fn generate_builder_dev_env_nix(
    cixfile: &Cixfile,
    builder_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    let builder = cixfile
        .builders
        .get(builder_name)
        .with_context(|| format!("unknown BUILDER {builder_name:?}"))?;
    let source_dir = source_dir.canonicalize()?;
    let primary = primary_namespace(cixfile)?;
    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    let imports = package_references(builder.imports.iter());
    writeln!(
        expression,
        "in universes.{}.mkShell {{ packages = [ {} ]; }}",
        nix_attr(primary),
        imports
            .iter()
            .map(|(namespace, attrpath)| package_expression(namespace, attrpath))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    Ok(expression)
}

pub fn generate_fetch_context_nix(
    cixfile: &Cixfile,
    fetch_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    let fetch = cixfile
        .fetches
        .get(fetch_name)
        .with_context(|| format!("unknown top-level FETCH {fetch_name:?}"))?;
    let source_dir = source_dir.canonicalize()?;
    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    let primary = primary_namespace(cixfile)?;
    writeln!(expression, "in {{")?;
    let refs = package_references([&fetch.command]);
    writeln!(
        expression,
        "  offers = [ (builtins.toString universes.{}.bash) {} ];",
        nix_attr(primary),
        refs.iter()
            .map(|(namespace, attrpath)| format!(
                "(builtins.toString {})",
                package_expression(namespace, attrpath)
            ))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    writeln!(
        expression,
        "  imports = [ \"${{universes.{}.bash}}\" ];",
        nix_attr(primary)
    )?;
    writeln!(
        expression,
        "  commands = [ {} ];",
        nix_template(&fetch.command)
    )?;
    writeln!(expression, "  copies = [];")?;
    writeln!(expression, "  environment = {{}};")?;
    writeln!(
        expression,
        "  universeIdentities = {};",
        universe_identities(cixfile, lock)
    )?;
    writeln!(expression, "}}")?;
    Ok(expression)
}

pub fn generate_fetch_offer_nix(
    cixfile: &Cixfile,
    fetch_name: &str,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    let fetch = cixfile
        .fetches
        .get(fetch_name)
        .with_context(|| format!("unknown top-level FETCH {fetch_name:?}"))?;
    let source_dir = source_dir.canonicalize()?;
    let mut expression = nix_prelude(cixfile, &source_dir, lock, system, snapshots)?;
    let primary = primary_namespace(cixfile)?;
    let refs = package_references([&fetch.command]);
    writeln!(
        expression,
        "in [ universes.{}.bash {} ]",
        nix_attr(primary),
        refs.iter()
            .map(|(namespace, attrpath)| package_expression(namespace, attrpath))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    Ok(expression)
}

fn nix_prelude(
    cixfile: &Cixfile,
    source_dir: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<String> {
    lock.validate_for(&cixfile.inputs)?;
    validate_snapshots(snapshots)?;
    let mut expression = String::new();
    writeln!(expression, "let")?;
    writeln!(
        expression,
        "  sourceRoot = builtins.path {{ path = {}; name = \"cix-source\"; }};",
        nix_string(
            source_dir
                .to_str()
                .context("Cixfile directory is not valid UTF-8")?
        )
    )?;
    for (index, (name, input)) in cixfile
        .inputs
        .iter()
        .filter(|(_, input)| !input.is_local() && input.kind != InputKind::Artifact)
        .enumerate()
    {
        let locked = &lock.inputs[name];
        writeln!(
            expression,
            "  input{index}Source = {};",
            fetch_tree(locked)?
        )?;
        let _ = input;
    }
    writeln!(expression, "  universes = {{")?;
    for (index, (name, input)) in cixfile
        .inputs
        .iter()
        .filter(|(_, input)| !input.is_local() && input.kind != InputKind::Artifact)
        .enumerate()
    {
        if input.kind == InputKind::PackageUniverse && input.overlays.is_empty() {
            writeln!(
                expression,
                "    {} = import input{index}Source {{ system = {}; }};",
                nix_attr(name),
                nix_string(system)
            )?;
        } else if input.kind == InputKind::PackageUniverse {
            writeln!(expression, "    {} =", nix_attr(name))?;
            writeln!(expression, "      let base = import input{index}Source;")?;
            writeln!(expression, "          args = if builtins.isFunction base then builtins.functionArgs base else throw {};", nix_string(&format!("FROM {name} with OVERLAY must import a function accepting overlays; wrap the base or use a full universe tree")))?;
            writeln!(
                expression,
                "      in if args ? overlays then base {{ system = {}; overlays = [",
                nix_string(system)
            )?;
            for overlay in &input.overlays {
                let error = nix_string(&format!("FROM {name} OVERLAY {overlay} must be a final: prev: function returning an attrset"));
                writeln!(expression, "        (final: prev: let overlay = import (sourceRoot + {}); in if builtins.isFunction overlay then let result = overlay final prev; in if builtins.isAttrs result then result else throw {} else throw {})", nix_string(overlay.trim_start_matches('.')), error, error)?;
            }
            writeln!(expression, "      ]; }} else throw {};", nix_string(&format!("FROM {name} with OVERLAY requires a base accepting an overlays argument; wrap the base or use a full universe tree")))?;
        }
    }
    writeln!(expression, "  }};")?;
    writeln!(expression, "  binders = {{")?;
    let mut remote_index = 0;
    for (name, input) in &cixfile.inputs {
        if input.kind == InputKind::Source {
            if input.is_local() {
                if input.url == "." {
                    writeln!(expression, "    {} = sourceRoot;", nix_attr(name))?;
                } else {
                    writeln!(
                        expression,
                        "    {} = sourceRoot + {};",
                        nix_attr(name),
                        nix_string(input.url.trim_start_matches("./"))
                    )?;
                }
            } else {
                writeln!(
                    expression,
                    "    {} = input{remote_index}Source;",
                    nix_attr(name)
                )?;
            }
        }
        if input.kind == InputKind::Artifact {
            let pin = lock
                .artifacts
                .get(&input.url)
                .with_context(|| format!("lock is missing cix-item FROM ref {:?}", input.url))?;
            writeln!(
                expression,
                "    {} = builtins.storePath {};",
                nix_attr(name),
                nix_string(&pin.store_path)
            )?;
        }
        if !input.is_local() && input.kind != InputKind::Artifact {
            remote_index += 1;
        }
    }
    for (name, snapshot) in snapshots {
        writeln!(
            expression,
            "    {} = builtins.storePath {};",
            nix_attr(name),
            nix_string(snapshot)
        )?;
    }
    writeln!(expression, "  }};")?;
    Ok(expression)
}

fn builder_templates(builder: &Builder) -> Vec<&Template> {
    builder
        .imports
        .iter()
        .chain(builder.steps.iter().flat_map(|step| match step {
            BuildStep::Env { .. } => vec![],
            BuildStep::Copy(copy) => vec![&copy.src],
            BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } => vec![command],
        }))
        .collect()
}

fn builder_command_templates(builder: &Builder) -> Vec<&Template> {
    builder
        .imports
        .iter()
        .chain(builder.steps.iter().filter_map(|step| match step {
            BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } => Some(command),
            BuildStep::Env { .. } | BuildStep::Copy(_) => None,
        }))
        .collect()
}

fn offer_expressions<'a>(
    package_refs: &BTreeSet<(String, String)>,
    templates: impl Iterator<Item = &'a Template>,
) -> String {
    let mut offers = package_refs
        .iter()
        .map(|(namespace, attrpath)| {
            format!(
                "(builtins.toString {})",
                package_expression(namespace, attrpath)
            )
        })
        .collect::<Vec<_>>();
    let binders = templates
        .flat_map(|template| &template.parts)
        .filter_map(|part| match part {
            TemplatePart::Binder { name, .. } => Some(name),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    offers.extend(
        binders
            .into_iter()
            .map(|name| format!("(builtins.toString binders.{})", nix_attr(name))),
    );
    offers.join(" ")
}

fn package_references<'a>(
    templates: impl IntoIterator<Item = &'a Template>,
) -> BTreeSet<(String, String)> {
    templates
        .into_iter()
        .flat_map(|template| &template.parts)
        .filter_map(|part| match part {
            TemplatePart::Package {
                namespace,
                attrpath,
                ..
            } => Some((namespace.clone(), attrpath.clone())),
            _ => None,
        })
        .collect()
}

fn validate_snapshots(snapshots: &BTreeMap<String, String>) -> Result<()> {
    for (name, snapshot) in snapshots {
        if !snapshot.starts_with("/nix/store/") {
            bail!("snapshot for binder {name:?} is not a Nix store path: {snapshot}");
        }
    }
    Ok(())
}

fn emit_copy(expression: &mut String, index: usize, copy: &Copy) -> Result<()> {
    writeln!(
        expression,
        "  if [ ! -e \"${{copy{index}}}\" ] && [ ! -L \"${{copy{index}}}\" ]; then echo {} >&2; exit 1; fi",
        nix_string(&format!("line {}: COPY source does not exist", copy.line))
    )?;
    if matches!(copy.mode, CopyMode::Link | CopyMode::LinkNormalized) {
        writeln!(
            expression,
            "  ln -s \"${{copy{index}}}\" \"$out/{}\"",
            shell_double_quoted(&copy.dst)
        )?;
    } else if copy.dst == "." {
        writeln!(
            expression,
            "  if [ -d \"${{copy{index}}}\" ]; then cp -a \"${{copy{index}}}/.\" \"$out/\"; else cp -a \"${{copy{index}}}\" \"$out/\"; fi"
        )?;
    } else {
        // Store directory modes are read-only; descendant assembly writes need a writable private copy.
        writeln!(
            expression,
            "  if [ -d \"${{copy{index}}}\" ]; then mkdir -p \"$out/{}\"; cp -a \"${{copy{index}}}/.\" \"$out/{}/\"; chmod -R u+w \"$out/{}\"; else cp -a \"${{copy{index}}}\" \"$out/{}\"; fi",
            shell_double_quoted(&copy.dst),
            shell_double_quoted(&copy.dst),
            shell_double_quoted(&copy.dst),
            shell_double_quoted(&copy.dst),
        )?;
    }
    Ok(())
}

fn nix_copy_source(template: &Template) -> String {
    match template.parts.as_slice() {
        [TemplatePart::Literal(path)] if path == "." => "\"${sourceRoot}\"".to_owned(),
        [TemplatePart::Literal(path)] if path.starts_with("/nix/store/") => nix_string(path),
        [TemplatePart::Literal(path)] => {
            format!("\"${{sourceRoot}}/{}\"", escape_nix_string(path))
        }
        _ => nix_template(template),
    }
}

fn nix_artifact_copy_source(copy: &Copy) -> String {
    let source = nix_copy_source(&copy.src);
    if copy.mode == CopyMode::LinkNormalized {
        format!("builtins.path {{ path = {source}; name = \"cix-copy\"; }}")
    } else {
        source
    }
}

fn nix_spec(artifact: &Artifact) -> Result<String> {
    if !artifact.kind.is_runnable() {
        bail!("ITEM blocks are content-only and do not have runtime manifests; see docs/cixfile.md#item");
    }
    let mounts = projected_mounts(artifact);
    let mut output = String::from("{ cixManifest = 0;");
    if let Some(kind) = artifact.kind.manifest_name() {
        write!(output, " kind = {};", nix_string(kind))?;
    }
    let service = nix_service(artifact, &mounts)?;
    output.push_str(service.trim_start_matches('{').trim_end_matches('}'));
    output.push_str(" }");
    Ok(output)
}

fn nix_service(artifact: &Artifact, mounts: &BTreeSet<String>) -> Result<String> {
    let service = &artifact.service;
    let mut output = String::from("{");
    write!(output, " start = {};", nix_command(&service.start))?;
    if !artifact.imports.is_empty() {
        write!(
            output,
            " mounts = builtins.attrNames (builtins.listToAttrs (map (name: {{ inherit name; value = null; }}) ([ {} ] ++ importMounts)));",
            mounts
                .iter()
                .map(|mount| nix_string(mount))
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    } else if !mounts.is_empty() {
        write!(
            output,
            " mounts = [ {} ];",
            mounts
                .iter()
                .map(|mount| nix_string(mount))
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    }
    if let Some(start_pre) = &service.start_pre {
        write!(output, " start_pre = {};", nix_command(start_pre))?;
    }
    {
        output.push_str(" env = {");
        if !service.env.contains_key("PATH") {
            write!(output, " {} = {{ default = \"bin\"; }};", nix_attr("PATH"))?;
        }
        for (name, env) in &service.env {
            write!(output, " {} = {{", nix_attr(name))?;
            if let Some(default) = &env.default {
                write!(output, " default = {};", nix_template(default))?;
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
            write!(output, " {} = {{", nix_attr(name))?;
            match port {
                Port::Env(variable) => write!(output, " env = {};", nix_string(variable))?,
                Port::Value(value) => write!(output, " value = {value};")?,
            }
            output.push_str(" protocol = \"tcp\"; };");
        }
        output.push_str(" };");
    }
    if !service.listeners.is_empty() {
        output.push_str(" listeners = {");
        for name in &service.listeners {
            write!(output, " {} = {{ type = \"stream\"; }};", nix_attr(name))?;
        }
        output.push_str(" };");
    }
    if let Some(readiness) = &service.readiness {
        write!(
            output,
            " readiness = {};",
            nix_probe(&readiness.probe, "timeout", &readiness.timeout)
        )?;
    }
    if let Some(liveness) = &service.liveness {
        write!(
            output,
            " liveness = {};",
            nix_probe(&liveness.probe, "interval", &liveness.interval)
        )?;
    }
    if !service.secrets.is_empty() {
        write!(
            output,
            " secrets = {{ {} }};",
            service
                .secrets
                .iter()
                .map(|(name, secret)| match &secret.as_env {
                    Some(as_env) =>
                        format!("{} = {{ as = {}; }};", nix_attr(name), nix_string(as_env)),
                    None => format!("{} = {{}};", nix_attr(name)),
                })
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    }
    if let Some(dirs) = nix_dirs(service) {
        write!(output, " dirs = {dirs};")?;
    }
    if !service.claims.is_empty() {
        write!(
            output,
            " claims = [ {} ];",
            service
                .claims
                .iter()
                .map(nix_claim)
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    }
    if let Some(shm) = &service.shm {
        write!(output, " shm = {};", nix_string(shm))?;
    }
    output.push_str(" }");
    Ok(output)
}

fn nix_probe(probe: &Probe, duration_name: &str, duration: &str) -> String {
    let (probe_type, target) = match probe {
        Probe::Http(target) => ("http", Some(target)),
        Probe::Tcp(target) => ("tcp", Some(target)),
        Probe::Notify => ("notify", None),
    };
    let target = target
        .map(|target| format!(" target = {};", nix_string(target)))
        .unwrap_or_default();
    format!(
        "{{ type = {};{target} {duration_name} = {}; }}",
        nix_string(probe_type),
        nix_string(duration)
    )
}

fn nix_dirs(service: &Service) -> Option<String> {
    let roles = [
        ("state", &service.dirs.state),
        ("cache", &service.dirs.cache),
        ("logs", &service.dirs.logs),
        ("config", &service.dirs.config),
        ("run", &service.dirs.run),
    ];
    if roles.iter().all(|(_, paths)| paths.is_empty()) && service.dirs.data.is_empty() {
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
    if !service.dirs.data.is_empty() {
        write!(
            output,
            " data = [ {} ];",
            service
                .dirs
                .data
                .iter()
                .map(|(path, ro)| format!("{{ path = {}; ro = {}; }}", nix_string(path), ro))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .unwrap();
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

fn nix_command(arguments: &[Template]) -> String {
    let Some(command) = bare_command(arguments) else {
        return nix_templates(arguments);
    };
    let mut output = format!("[ {}", nix_string(&format!("bin/{command}")));
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
            TemplatePart::Package {
                namespace,
                attrpath,
                line,
            } => {
                output.push_str("${builtins.addErrorContext ");
                output.push_str(&nix_string(&format!(
                    "Cixfile line {line}: resolving {namespace}.{attrpath}"
                )));
                output.push(' ');
                output.push_str(&package_expression(namespace, attrpath));
                output.push('}');
            }
            TemplatePart::Binder { name, .. } => {
                output.push_str("${binders.");
                output.push_str(&nix_attr(name));
                output.push('}');
            }
            TemplatePart::InputMetadata {
                namespace,
                attribute,
                line,
            } => {
                output.push_str("${builtins.throw ");
                output.push_str(&nix_string(&format!(
                    "Cixfile line {line}: FROM metadata {namespace}.{attribute} was not resolved from Cixfile.lock"
                )));
                output.push('}');
            }
        }
    }
    output.push('"');
    output
}

fn package_expression(namespace: &str, attrpath: &str) -> String {
    let mut expression = format!("universes.{}", nix_attr(namespace));
    for component in attrpath.split('.') {
        expression.push('.');
        expression.push_str(&nix_attr(component));
    }
    expression
}

fn github_repository(url: &str) -> Result<(&str, &str)> {
    let path = url
        .strip_prefix("github:")
        .with_context(|| format!("unsupported FROM URL {url:?}; expected github:owner/repo"))?;
    let mut components = path.split('/');
    let owner = components.next().filter(|value| !value.is_empty());
    let repo = components.next().filter(|value| !value.is_empty());
    match (owner, repo) {
        (Some(owner), Some(repo)) => Ok((owner, repo)),
        _ => bail!("invalid github FROM URL {url:?}"),
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

fn universe_identities(cixfile: &Cixfile, lock: &LockFile) -> String {
    let identities = cixfile
        .inputs
        .iter()
        .filter(|(_, input)| input.kind == InputKind::PackageUniverse)
        .map(|(name, input)| {
            let locked = &lock.inputs[name];
            let overlays = input
                .overlays
                .iter()
                .map(|overlay| {
                    format!(
                        "(builtins.hashFile \"sha256\" (sourceRoot + {}))",
                        nix_string(overlay.trim_start_matches('.'))
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "{} = builtins.hashString \"sha256\" (builtins.toJSON {{ url = {}; rev = {}; narHash = {}; overlays = [ {} ]; }});",
                nix_attr(name),
                nix_string(&locked.url),
                nix_string(&locked.rev),
                nix_string(&locked.nar_hash),
                overlays,
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{{ {identities} }}")
}

fn primary_namespace(cixfile: &Cixfile) -> Result<&str> {
    cixfile
        .inputs
        .iter()
        .find(|(_, input)| input.kind == InputKind::PackageUniverse)
        .map(|(name, _)| name.as_str())
        .context("Cixfile has no package-universe FROM input")
}

fn only_artifact(cixfile: &Cixfile) -> Result<(&str, &Artifact)> {
    if cixfile.artifacts.len() != 1 {
        bail!(
            "this operation requires exactly one artifact block, found {}",
            cixfile.artifacts.len()
        );
    }
    let (name, artifact) = cixfile.artifacts.first_key_value().expect("one artifact");
    Ok((name, artifact))
}

fn artifact_directories(artifact: &Artifact) -> BTreeSet<String> {
    let mut directories = artifact
        .copies
        .iter()
        .map(|copy| &copy.dst)
        .chain(artifact.assembly.iter().map(|assembly| match assembly {
            Assembly::File { dst, .. } | Assembly::Link { dst, .. } => dst,
        }))
        .filter_map(|destination| {
            Path::new(destination)
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| parent.to_string_lossy().into_owned())
        })
        .collect::<BTreeSet<_>>();
    if !artifact.imports.is_empty() {
        directories.extend(["bin".to_owned(), "etc".to_owned(), "share".to_owned()]);
    }
    directories
}

fn projected_mounts(artifact: &Artifact) -> BTreeSet<String> {
    let copy_destinations = artifact
        .copies
        .iter()
        .filter(|copy| copy.dst != ".")
        .map(|copy| format!("/{}", copy.dst));
    let assembly_destinations = artifact
        .assembly
        .iter()
        .map(|assembly| match assembly {
            Assembly::File { dst, .. } | Assembly::Link { dst, .. } => dst,
        })
        .map(|dst| format!("/{dst}"));
    copy_destinations
        .chain(assembly_destinations)
        .filter_map(|destination| {
            let path = Path::new(&destination);
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
        .collect::<BTreeSet<_>>()
}

fn literal_spec(artifact: &Artifact) -> Result<Value> {
    if !artifact.kind.is_runnable() {
        bail!("ITEM blocks are content-only and do not have runtime manifests; see docs/cixfile.md#item");
    }
    let mounts = projected_mounts(artifact);
    let Value::Object(mut value) = literal_service(artifact, &mounts)? else {
        unreachable!("artifact literal is an object");
    };
    value.insert("cixManifest".to_owned(), Value::from(0));
    if let Some(kind) = artifact.kind.manifest_name() {
        value.insert("kind".to_owned(), Value::String(kind.to_owned()));
    }
    Ok(Value::Object(value))
}

fn literal_service(artifact: &Artifact, mounts: &BTreeSet<String>) -> Result<Value> {
    let service = &artifact.service;
    let mut value = Map::new();
    value.insert("start".into(), literal_command(&service.start)?);
    if !mounts.is_empty() {
        value.insert(
            "mounts".into(),
            Value::Array(mounts.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(start_pre) = &service.start_pre {
        value.insert("start_pre".into(), literal_command(start_pre)?);
    }
    {
        let mut envs = Map::new();
        if !service.env.contains_key("PATH") {
            envs.insert(
                "PATH".into(),
                Value::Object(Map::from_iter([(
                    "default".into(),
                    Value::String("bin".into()),
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
    if let Some(readiness) = &service.readiness {
        value.insert(
            "readiness".into(),
            literal_probe(&readiness.probe, "timeout", &readiness.timeout),
        );
    }
    if let Some(liveness) = &service.liveness {
        value.insert(
            "liveness".into(),
            literal_probe(&liveness.probe, "interval", &liveness.interval),
        );
    }
    if !service.secrets.is_empty() {
        value.insert(
            "secrets".into(),
            Value::Object(
                service
                    .secrets
                    .iter()
                    .map(|(name, secret)| {
                        let mut declaration = Map::new();
                        if let Some(as_env) = &secret.as_env {
                            declaration.insert("as".into(), Value::String(as_env.clone()));
                        }
                        (name.clone(), Value::Object(declaration))
                    })
                    .collect(),
            ),
        );
    }
    let dirs = literal_dirs(service);
    if !dirs.is_empty() {
        value.insert("dirs".into(), Value::Object(dirs));
    }
    if !service.claims.is_empty() {
        value.insert(
            "claims".into(),
            Value::Array(service.claims.iter().map(literal_claim).collect()),
        );
    }
    if let Some(shm) = &service.shm {
        value.insert("shm".into(), Value::String(shm.clone()));
    }
    Ok(Value::Object(value))
}

fn literal_probe(probe: &Probe, duration_name: &str, duration: &str) -> Value {
    let (probe_type, target) = match probe {
        Probe::Http(target) => ("http", Some(target)),
        Probe::Tcp(target) => ("tcp", Some(target)),
        Probe::Notify => ("notify", None),
    };
    let mut fields = Map::from_iter([
        ("type".into(), Value::String(probe_type.into())),
        (duration_name.into(), Value::String(duration.into())),
    ]);
    if let Some(target) = target {
        fields.insert("target".into(), Value::String(target.clone()));
    }
    Value::Object(fields)
}

fn nix_claim(claim: &Claim) -> String {
    match claim {
        Claim::Named(name) => nix_string(name),
        Claim::Device(path) => format!("{{ device = {}; }}", nix_string(path)),
    }
}

fn literal_claim(claim: &Claim) -> Value {
    match claim {
        Claim::Named(name) => Value::String(name.clone()),
        Claim::Device(path) => Value::Object(Map::from_iter([(
            "device".into(),
            Value::String(path.clone()),
        )])),
    }
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
    if !service.dirs.data.is_empty() {
        dirs.insert(
            "data".into(),
            Value::Array(
                service
                    .dirs
                    .data
                    .iter()
                    .map(|(path, ro)| {
                        Value::Object(Map::from_iter([
                            ("path".into(), Value::String(path.clone())),
                            ("ro".into(), Value::Bool(*ro)),
                        ]))
                    })
                    .collect(),
            ),
        );
    }
    dirs
}

fn literal_command(arguments: &[Template]) -> Result<Value> {
    let mut values = arguments
        .iter()
        .map(literal_template)
        .collect::<Result<Vec<_>>>()?;
    if let Some(command) = bare_command(arguments) {
        values[0] = format!("bin/{command}");
    }
    Ok(Value::Array(
        values.into_iter().map(Value::String).collect(),
    ))
}

fn literal_template(template: &Template) -> Result<String> {
    template.literal_value().with_context(|| {
        "cannot render package or binder interpolation as JSON without evaluating generated Nix"
    })
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

#[cfg(all(test, any()))]
mod tests {
    use super::*;
    use cix_cixfile::parse;

    fn fixture_lock() -> LockFile {
        LockFile {
            inputs: BTreeMap::from([(
                "pkgs".into(),
                InputLock {
                    url: "github:NixOS/nixpkgs/nixos-unstable".into(),
                    rev: "0123456789abcdef".into(),
                    nar_hash: "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
                    rev_count: None,
                    last_modified: None,
                },
            )]),
            artifacts: BTreeMap::new(),
            fetches: BTreeMap::new(),
            memo: BTreeMap::new(),
            step_memo: BTreeMap::new(),
            dev_envs: BTreeMap::new(),
            outputs: BTreeMap::new(),
        }
    }

    #[test]
    fn golden_cixfile_generates_expected_spec() {
        let cixfile = parse(include_str!("../../cix-cixfile/tests/golden/Cixfile")).unwrap();
        let actual = generate_spec_json(&cixfile).unwrap();
        assert_eq!(
            actual,
            include_str!("../../cix-cixfile/tests/golden/cix-manifest.json")
        );
    }

    #[test]
    fn emits_kind_and_unified_copy_sources() {
        let directory = tempfile::tempdir().unwrap();
        let cixfile = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nAPP job\nCOPY ${src}/payload /bin/payload\nSTART /bin/true\n",
        )
        .unwrap();
        let nix =
            generate_nix(&cixfile, directory.path(), &fixture_lock(), "x86_64-linux").unwrap();
        assert!(nix.contains("kind = \"app\";"), "{nix}");
        assert!(
            nix.contains("copy0 = \"${binders.\"src\"}/payload\";"),
            "{nix}"
        );
    }

    #[test]
    fn service_manifest_omits_kind_and_app_emits_it() {
        let service = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n",
        )
        .unwrap();
        let spec = generate_spec_json(&service).unwrap();
        assert!(!spec.contains("\"kind\""), "{spec}");
        assert!(
            spec.contains("\"PATH\": {\n      \"default\": \"bin\""),
            "{spec}"
        );

        let app =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nAPP job\nSTART /bin/true\n")
                .unwrap();
        let manifest = generate_spec_json(&app).unwrap();
        assert!(manifest.contains("\"kind\": \"app\""), "{manifest}");
        assert!(manifest.contains("\"start\""), "{manifest}");
        assert!(
            manifest.contains("\"PATH\": {\n      \"default\": \"bin\""),
            "{manifest}"
        );

        let explicit =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nENV PATH = /tools/bin\nSTART /bin/true\n")
                .unwrap();
        let explicit_spec = generate_spec_json(&explicit).unwrap();
        assert!(
            explicit_spec.contains("\"default\": \"/tools/bin\""),
            "{explicit_spec}"
        );
        assert!(!explicit_spec.contains("bin:/tools/bin"), "{explicit_spec}");
    }

    #[test]
    fn absolute_artifact_destinations_keep_the_pre_d66_manifest_shape() {
        let cixfile = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nCOPY payload /share/payload\nFILE /etc/fixture.conf <<EOF\nvalue\nEOF\nLINK /nix/store/tool /bin/tool\nSTART /bin/tool\n",
        )
        .unwrap();
        assert_eq!(cixfile.artifacts["fixture"].copies[0].dst, "share/payload");
        assert!(matches!(
            &cixfile.artifacts["fixture"].assembly[..],
            [Assembly::File { dst: file, .. }, Assembly::Link { dst: link, .. }]
                if file == "etc/fixture.conf" && link == "bin/tool"
        ));
        let manifest = generate_spec_json(&cixfile).unwrap();
        let before_d66 = r#"{
  "cixManifest": 0,
  "env": {
    "PATH": {
      "default": "bin"
    }
  },
  "start": [
    "/bin/tool"
  ],
  "mounts": [
    "/bin/tool",
    "/etc/fixture.conf",
    "/share/payload"
  ]
}
"#;
        assert_eq!(manifest, before_d66);
    }

    #[test]
    fn remote_source_from_is_pinned_and_exposed_as_a_tree() {
        let directory = tempfile::tempdir().unwrap();
        let cixfile = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM github:owner/repository/deadbeef AS src\nSERVICE data\nCOPY ${src}/payload /payload\nSTART /bin/true\n",
        )
        .unwrap();
        let mut lock = fixture_lock();
        lock.inputs.insert(
            "src".into(),
            InputLock {
                url: "github:owner/repository/deadbeef".into(),
                rev: "deadbeef".into(),
                nar_hash: "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".into(),
                rev_count: None,
                last_modified: None,
            },
        );
        let nix = generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux").unwrap();
        assert!(nix.contains("\"src\" = input1Source;"), "{nix}");
        assert!(
            nix.contains("copy0 = \"${binders.\"src\"}/payload\";"),
            "{nix}"
        );
    }
}
