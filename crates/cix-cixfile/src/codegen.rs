use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

use cix_build::{
    Artifact, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, CopyMode, InputKind, InputLock,
    LockFile, NodeCommand, PortSource, Probe, Protocol, Template, TemplatePart,
};

pub(crate) struct Codegen;

impl cix_build::EvaluationCodegen for Codegen {
    fn fetch_context(
        &self,
        cixfile: &Cixfile,
        fetch_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String> {
        generate_fetch_context_nix(cixfile, fetch_name, source_dir, lock, system, snapshots)
    }

    fn fetch_offers(
        &self,
        cixfile: &Cixfile,
        fetch_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String> {
        generate_fetch_offer_nix(cixfile, fetch_name, source_dir, lock, system, snapshots)
    }

    fn builder_context(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String> {
        generate_builder_context_nix(cixfile, builder_name, source_dir, lock, system, snapshots)
    }

    fn builder_offers(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String> {
        generate_builder_offer_nix(cixfile, builder_name, source_dir, lock, system, snapshots)
    }

    fn builder_dev_environment(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String> {
        generate_builder_dev_env_nix(cixfile, builder_name, source_dir, lock, system, snapshots)
    }
}

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
    manifest_contract(artifact, selected_args(cixfile), literal_template)?.to_canonical_json()
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
        writeln!(expression, "  spec = {};", nix_spec(cixfile, artifact)?)?;
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
        writeln!(expression, "  manifestJson = builtins.toJSON spec;")?;
        writeln!(
            expression,
            "  manifestFile = universes.{}.runCommand \"cix-manifest.json\" {{ nativeBuildInputs = [ universes.{}.jq ]; inherit manifestJson; }} ''",
            nix_attr(primary),
            nix_attr(primary),
        )?;
        writeln!(
            expression,
            "    printf '%s\\n' \"$manifestJson\" | jq . > \"$out\""
        )?;
        writeln!(expression, "  '';")?;
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
    let nodes = builder
        .steps
        .iter()
        .filter_map(|step| match step {
            BuildStep::Fetch {
                command,
                environment,
                ignored_evidence,
                ..
            }
            | BuildStep::Run {
                command,
                environment,
                ignored_evidence,
                ..
            } => Some(nix_resolved_node(command, environment, ignored_evidence)),
            BuildStep::Env { .. } | BuildStep::Copy(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    writeln!(expression, "  nodes = [ {nodes} ];")?;
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
    let fetch_templates = fetch
        .command
        .templates()
        .into_iter()
        .chain(fetch.environment.values());
    let refs = package_references(fetch_templates);
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
        "  nodes = [ {} ];",
        nix_resolved_node(&fetch.command, &fetch.environment, &fetch.ignored_evidence)
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
    let refs = package_references(
        fetch
            .command
            .templates()
            .into_iter()
            .chain(fetch.environment.values()),
    );
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
        .chain(builder.steps.iter().flat_map(|step| {
            match step {
                BuildStep::Env { .. } => vec![],
                BuildStep::Copy(copy) => vec![&copy.src],
                BuildStep::Fetch {
                    command,
                    environment,
                    ..
                }
                | BuildStep::Run {
                    command,
                    environment,
                    ..
                } => command
                    .templates()
                    .into_iter()
                    .chain(environment.values())
                    .collect(),
            }
        }))
        .collect()
}

fn builder_command_templates(builder: &Builder) -> Vec<&Template> {
    builder
        .imports
        .iter()
        .chain(builder.steps.iter().flat_map(|step| {
            match step {
                BuildStep::Fetch {
                    command,
                    environment,
                    ..
                }
                | BuildStep::Run {
                    command,
                    environment,
                    ..
                } => command
                    .templates()
                    .into_iter()
                    .chain(environment.values())
                    .collect(),
                BuildStep::Env { .. } | BuildStep::Copy(_) => vec![],
            }
        }))
        .collect()
}

fn nix_resolved_node(
    command: &NodeCommand,
    environment: &BTreeMap<String, Template>,
    ignored_evidence: &BTreeSet<String>,
) -> String {
    let command = match command {
        NodeCommand::Legacy(command) => format!(
            "{{ kind = \"legacy\"; command = {}; }}",
            nix_template(command)
        ),
        NodeCommand::Argv(argv) => {
            format!("{{ kind = \"argv\"; argv = {}; }}", nix_templates(argv))
        }
        NodeCommand::Heredoc { interpreter, body } => format!(
            "{{ kind = \"heredoc\"; interpreter = {}; body = {}; }}",
            nix_template(interpreter),
            nix_template(body)
        ),
    };
    let environment = environment
        .iter()
        .map(|(name, value)| format!("{} = {};", nix_attr(name), nix_template(value)))
        .collect::<Vec<_>>()
        .join(" ");
    let ignored_evidence = ignored_evidence
        .iter()
        .map(|path| nix_string(path))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "{{ command = {command}; environment = {{ {environment} }}; ignoredEvidence = [ {ignored_evidence} ]; }}"
    )
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

fn nix_spec(cixfile: &Cixfile, artifact: &Artifact) -> Result<String> {
    let contract = manifest_contract(artifact, selected_args(cixfile), |template| {
        Ok(template.clone())
    })?;
    let service = contract
        .services
        .first_key_value()
        .map(|(_, service)| service)
        .context("bare manifest has no def-node")?;
    let mut output = String::from("{ cixManifest = 0;");
    if !contract.build_args.is_empty() {
        output.push_str(" buildArgs = {");
        for (name, value) in &contract.build_args {
            write!(output, " {} = {};", nix_attr(name), nix_string(value))?;
        }
        output.push_str(" };");
    }
    if contract.kind != cix_manifest::ManifestKind::Service {
        write!(output, " kind = \"app\";")?;
    }
    let service = nix_service(
        service,
        !artifact.imports.is_empty(),
        !artifact.service.env.contains_key("PATH"),
    )?;
    output.push_str(service.trim_start_matches('{').trim_end_matches('}'));
    output.push_str(" }");
    Ok(output)
}

fn nix_service(
    service: &cix_manifest::Service<Template>,
    import_mounts: bool,
    implicit_path: bool,
) -> Result<String> {
    let mut output = String::from("{");
    write!(output, " start = {};", nix_templates(&service.start))?;
    let mounts = service.mounts.as_deref().unwrap_or_default();
    if import_mounts {
        write!(
            output,
            " mounts = builtins.attrNames (builtins.listToAttrs (map (name: {{ inherit name; value = null; }}) ([ {} ] ++ importMounts)));",
            mounts
                .iter()
                .map(|mount| nix_string(&mount.to_string_lossy()))
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    } else if !mounts.is_empty() {
        write!(
            output,
            " mounts = [ {} ];",
            mounts
                .iter()
                .map(|mount| nix_string(&mount.to_string_lossy()))
                .collect::<Vec<_>>()
                .join(" ")
        )?;
    }
    if let Some(start_pre) = &service.start_pre {
        write!(output, " start_pre = {};", nix_templates(start_pre))?;
    }
    {
        output.push_str(" env = {");
        if implicit_path {
            let path = &service.env["PATH"];
            write!(output, " {} = {{", nix_attr("PATH"))?;
            if let Some(default) = &path.default {
                write!(output, " default = {};", nix_template(default))?;
            }
            output.push_str(" };");
        }
        for (name, env) in &service.env {
            if implicit_path && name == "PATH" {
                continue;
            }
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
            match (&port.env, port.value) {
                (Some(variable), None) => write!(output, " env = {};", nix_string(variable))?,
                (None, Some(value)) => write!(output, " value = {value};")?,
                _ => unreachable!("language port source is exactly one of env or value"),
            }
            let protocol = match port.protocol {
                cix_manifest::Protocol::Tcp => "tcp",
                cix_manifest::Protocol::Udp => "udp",
            };
            write!(output, " protocol = {protocol:?}; }};")?;
        }
        output.push_str(" };");
    }
    if !service.listeners.is_empty() {
        output.push_str(" listeners = {");
        for name in service.listeners.keys() {
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
    if let Some(stop_signal) = &service.stop_signal {
        write!(output, " stopSignal = {};", nix_string(stop_signal))?;
    }
    output.push_str(" }");
    Ok(output)
}

fn nix_probe(probe: &cix_manifest::Probe, duration_name: &str, duration: &str) -> String {
    let probe_type = match probe.probe_type {
        cix_manifest::ProbeType::Http => "http",
        cix_manifest::ProbeType::Tcp => "tcp",
        cix_manifest::ProbeType::Notify => "notify",
    };
    let target = probe
        .target
        .as_ref()
        .map(|target| format!(" target = {};", nix_string(target)))
        .unwrap_or_default();
    format!(
        "{{ type = {};{target} {duration_name} = {}; }}",
        nix_string(probe_type),
        nix_string(duration)
    )
}

fn nix_dirs(service: &cix_manifest::Service<Template>) -> Option<String> {
    let roles = [
        ("state", service.dirs.state.as_slice()),
        ("cache", service.dirs.cache.as_slice()),
        ("logs", service.dirs.logs.as_slice()),
        ("config", service.dirs.config.as_slice()),
        ("run", service.dirs.run.as_deref().unwrap_or_default()),
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
                    .map(|path| nix_string(&path.to_string_lossy()))
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
                .map(|data| {
                    format!(
                        "{{ path = {}; ro = {}; }}",
                        nix_string(&data.path.to_string_lossy()),
                        data.ro
                    )
                })
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
    if artifact.kind.is_runnable() {
        directories.extend(
            artifact
                .service
                .dirs
                .state
                .iter()
                .chain(&artifact.service.dirs.cache)
                .chain(&artifact.service.dirs.logs)
                .chain(&artifact.service.dirs.config)
                .chain(&artifact.service.dirs.run)
                .filter_map(|path| path.strip_prefix('/'))
                .map(str::to_owned),
        );
    }
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

fn manifest_contract<T>(
    artifact: &Artifact,
    build_args: BTreeMap<String, String>,
    render_template: impl Fn(&Template) -> Result<T>,
) -> Result<cix_manifest::Spec<T>> {
    if !artifact.kind.is_runnable() {
        bail!("ITEM blocks are content-only and do not have runtime manifests; see docs/cixfile.md#item");
    }
    let service = &artifact.service;
    let mut env = BTreeMap::new();
    if !service.env.contains_key("PATH") {
        env.insert(
            "PATH".into(),
            cix_manifest::Env {
                legacy_type: None,
                default: Some(render_template(&Template::literal("bin"))?),
                required: false,
                secret: false,
            },
        );
    }
    for (name, declaration) in &service.env {
        env.insert(
            name.clone(),
            cix_manifest::Env {
                legacy_type: None,
                default: declaration
                    .default
                    .as_ref()
                    .map(&render_template)
                    .transpose()?,
                required: declaration.required,
                secret: declaration.secret,
            },
        );
    }
    let contract = cix_manifest::Service {
        start: contract_command(&service.start, &render_template)?,
        mounts: (!projected_mounts(artifact).is_empty()).then(|| {
            projected_mounts(artifact)
                .into_iter()
                .map(PathBuf::from)
                .collect()
        }),
        start_pre: service
            .start_pre
            .as_ref()
            .map(|command| contract_command(command, &render_template))
            .transpose()?,
        env,
        secrets: service
            .secrets
            .iter()
            .map(|(name, secret)| {
                (
                    name.clone(),
                    cix_manifest::Secret {
                        as_env: secret.as_env.clone(),
                    },
                )
            })
            .collect(),
        ports: service
            .ports
            .iter()
            .map(|(name, port)| {
                let (env, value) = match &port.source {
                    PortSource::Env(variable) => (Some(variable.clone()), None),
                    PortSource::Value(value) => (None, Some(*value)),
                };
                (
                    name.clone(),
                    cix_manifest::Port {
                        env,
                        value,
                        protocol: match port.protocol {
                            Protocol::Tcp => cix_manifest::Protocol::Tcp,
                            Protocol::Udp => cix_manifest::Protocol::Udp,
                        },
                    },
                )
            })
            .collect(),
        listeners: service
            .listeners
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    cix_manifest::Listener {
                        listener_type: "stream".into(),
                    },
                )
            })
            .collect(),
        dirs: cix_manifest::Dirs {
            state: service.dirs.state.iter().map(PathBuf::from).collect(),
            cache: service.dirs.cache.iter().map(PathBuf::from).collect(),
            logs: service.dirs.logs.iter().map(PathBuf::from).collect(),
            config: service.dirs.config.iter().map(PathBuf::from).collect(),
            run: (!service.dirs.run.is_empty())
                .then(|| service.dirs.run.iter().map(PathBuf::from).collect()),
            data: service
                .dirs
                .data
                .iter()
                .map(|(path, ro)| cix_manifest::DataDir {
                    path: PathBuf::from(path),
                    ro: *ro,
                })
                .collect(),
        },
        readiness: service
            .readiness
            .as_ref()
            .map(|readiness| cix_manifest::Readiness {
                probe: manifest_probe(&readiness.probe),
                timeout: readiness.timeout.clone(),
            }),
        liveness: service
            .liveness
            .as_ref()
            .map(|liveness| cix_manifest::Liveness {
                probe: manifest_probe(&liveness.probe),
                interval: liveness.interval.clone(),
            }),
        network: None,
        claims: service.claims.iter().map(manifest_claim).collect(),
        shm: service.shm.clone(),
        stop_signal: service.stop_signal.clone(),
        jit: None,
        egress: false,
    };
    Ok(cix_manifest::Spec {
        cix_manifest: 0,
        kind: match artifact.kind {
            cix_build::ArtifactKind::App => cix_manifest::ManifestKind::App,
            cix_build::ArtifactKind::Service => cix_manifest::ManifestKind::Service,
            cix_build::ArtifactKind::Item => unreachable!("items were rejected"),
        },
        build_args,
        services: BTreeMap::from([("artifact".into(), contract)]),
    })
}

fn selected_args(cixfile: &Cixfile) -> BTreeMap<String, String> {
    cixfile
        .args
        .iter()
        .map(|(name, argument)| (name.clone(), argument.selected.clone()))
        .collect()
}

fn contract_command<T>(
    arguments: &[Template],
    render_template: &impl Fn(&Template) -> Result<T>,
) -> Result<Vec<T>> {
    let bare = bare_command(arguments);
    arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            if index == 0 {
                if let Some(command) = &bare {
                    return render_template(&Template::literal(format!("bin/{command}")));
                }
            }
            render_template(argument)
        })
        .collect()
}

fn manifest_probe(probe: &Probe) -> cix_manifest::Probe {
    let (probe_type, target) = match probe {
        Probe::Http(target) => (cix_manifest::ProbeType::Http, Some(target.clone())),
        Probe::Tcp(target) => (cix_manifest::ProbeType::Tcp, Some(target.clone())),
        Probe::Notify => (cix_manifest::ProbeType::Notify, None),
    };
    cix_manifest::Probe { probe_type, target }
}

fn manifest_claim(claim: &Claim) -> cix_manifest::Claim {
    match claim {
        Claim::Named(name) => cix_manifest::Claim::Named(name.clone()),
        Claim::Device(path) => cix_manifest::Claim::Device(cix_manifest::DeviceClaim {
            device: PathBuf::from(path),
        }),
    }
}

fn nix_claim(claim: &cix_manifest::Claim) -> String {
    match claim {
        cix_manifest::Claim::Named(name) => nix_string(name),
        cix_manifest::Claim::Device(device) => format!(
            "{{ device = {}; }}",
            nix_string(&device.device.to_string_lossy())
        ),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

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
            builder_dev_envs: BTreeMap::new(),
            eval_plan: None,
            outputs: BTreeMap::new(),
        }
    }

    #[test]
    fn golden_cixfile_generates_expected_spec() {
        let cixfile = parse(include_str!("../tests/golden/Cixfile")).unwrap();
        let actual = generate_spec_json(&cixfile).unwrap();
        let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../tests/golden/cix-manifest.json")).unwrap();
        assert_eq!(actual, expected);
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

        let explicit = parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nENV PATH=/tools/bin\nSTART /bin/true\n").unwrap();
        let explicit_spec = generate_spec_json(&explicit).unwrap();
        assert!(
            explicit_spec.contains("\"default\": \"/tools/bin\""),
            "{explicit_spec}"
        );
        assert!(!explicit_spec.contains("bin:/tools/bin"), "{explicit_spec}");
    }

    #[test]
    fn absolute_artifact_destinations_keep_the_projected_manifest_shape() {
        let cixfile = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nCOPY payload /share/payload\nFILE /etc/fixture.conf <<EOF\nvalue\nEOF\nCOPY /nix/store/tool /bin/tool\nSTART /bin/tool\n",
        )
        .unwrap();
        assert_eq!(cixfile.artifacts["fixture"].copies[0].dst, "share/payload");
        assert!(matches!(
            &cixfile.artifacts["fixture"].assembly[..],
            [Assembly::File { dst: file, .. }] if file == "etc/fixture.conf"
        ));
        assert_eq!(cixfile.artifacts["fixture"].copies[1].dst, "bin/tool");
        let manifest = generate_spec_json(&cixfile).unwrap();
        let before_d66 = r#"{
  "cixManifest": 0,
  "env": {
    "PATH": {
      "default": "bin"
    }
  },
  "mounts": [
    "/bin/tool",
    "/etc/fixture.conf",
    "/share/payload"
  ],
  "start": [
    "/bin/tool"
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
