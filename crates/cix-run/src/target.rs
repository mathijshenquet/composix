use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use cix_common::Ref;

use crate::spec::{ManifestKind, Service, Spec};

#[derive(Debug)]
struct Target {
    output: PathBuf,
    requested_service: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ResolvedService {
    pub output: PathBuf,
    pub name: String,
    pub kind: ManifestKind,
    pub service: Service,
}

pub(crate) fn resolve_service(state_directory: &Path, input: &str) -> Result<ResolvedService> {
    let target = resolve_target(state_directory, input)?;
    let spec = Spec::load(&target.output)?;
    match spec.select_service(target.requested_service.as_deref()) {
        Ok((name, service)) => Ok(ResolvedService {
            output: target.output,
            name: name.to_owned(),
            kind: spec.kind,
            service: service.clone(),
        }),
        Err(original_error) if target.requested_service.is_none() => {
            let Some((installable, service_name)) = split_single_hash(input) else {
                return Err(original_error);
            };
            let output = resolve_installable(state_directory, installable)?;
            let fallback_spec = Spec::load(&output)?;
            let (name, service) = fallback_spec.select_service(Some(service_name))?;
            Ok(ResolvedService {
                output,
                name: name.to_owned(),
                kind: fallback_spec.kind,
                service: service.clone(),
            })
        }
        Err(error) => Err(error),
    }
}

fn resolve_target(state_directory: &Path, input: &str) -> Result<Target> {
    if input.starts_with("/nix/store/") || input.matches('#').count() >= 2 {
        if let Some((installable, service)) = input.rsplit_once('#') {
            return Ok(Target {
                output: resolve_installable(state_directory, installable)?,
                requested_service: Some(service.to_owned()),
            });
        }
    }
    Ok(Target {
        output: resolve_installable(state_directory, input)?,
        requested_service: None,
    })
}

pub(crate) fn split_single_hash(input: &str) -> Option<(&str, &str)> {
    if input.matches('#').count() == 1 {
        input.rsplit_once('#')
    } else {
        None
    }
}

pub fn resolve_installable(state_directory: &Path, installable: &str) -> Result<PathBuf> {
    if installable.is_empty() {
        bail!("installable must not be empty");
    }
    let direct_path = PathBuf::from(installable);
    if direct_path.starts_with("/nix/store/") && direct_path.exists() {
        return Ok(direct_path);
    }

    match Ref::parse(installable) {
        Ok(reference) => match cix_index::resolve_with(
            &cix_index::Store::open(state_directory.to_owned())?,
            installable,
        ) {
            Ok(output) => return Ok(PathBuf::from(output.store_path)),
            Err(error) if reference.root_url.is_some() => {
                return Err(error).with_context(|| {
                    format!("failed to resolve qualified cix ref {installable:?}")
                });
            }
            Err(_) => {}
        },
        Err(error) if Ref::looks_like_untagged_ref(installable) => return Err(error),
        Err(_) => {}
    }

    let output = nix_build(installable)?;
    if !output.status.success() {
        bail!(
            "failed to resolve installable {installable:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let paths = String::from_utf8(output.stdout).context("nix emitted a non-UTF-8 store path")?;
    let paths = paths
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if paths.len() != 1 {
        bail!(
            "installable {installable:?} resolved to {} outputs; cix run needs exactly one",
            paths.len()
        );
    }
    let path = PathBuf::from(paths[0]);
    if !path.starts_with("/nix/store/") {
        bail!(
            "installable {installable:?} resolved outside the Nix store: {}",
            path.display()
        );
    }
    Ok(path)
}

fn nix_build(installable: &str) -> Result<Output> {
    nix_command(&["build", "--no-link", "--print-out-paths", "--", installable])
        .with_context(|| format!("failed to invoke nix for installable {installable:?}"))
}

fn nix_command(args: &[&str]) -> Result<Output> {
    let invoke = |program: &Path| Command::new(program).args(args).output();
    match invoke(Path::new("nix")) {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            invoke(Path::new("/nix/var/nix/profiles/default/bin/nix"))
                .context("failed to invoke fallback nix executable")
        }
        Err(error) => Err(error).context("failed to invoke nix"),
    }
}
