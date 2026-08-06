use std::path::PathBuf;

use anyhow::Result;

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

pub trait InstallableResolver {
    fn resolve_installable(&self, installable: &str) -> Result<PathBuf>;
}

pub(crate) fn resolve_service(
    resolver: &dyn InstallableResolver,
    input: &str,
) -> Result<ResolvedService> {
    let target = resolve_target(resolver, input)?;
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
            let output = resolver.resolve_installable(installable)?;
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

fn resolve_target(resolver: &dyn InstallableResolver, input: &str) -> Result<Target> {
    if input.starts_with("/nix/store/") || input.matches('#').count() >= 2 {
        if let Some((installable, service)) = input.rsplit_once('#') {
            return Ok(Target {
                output: resolver.resolve_installable(installable)?,
                requested_service: Some(service.to_owned()),
            });
        }
    }
    Ok(Target {
        output: resolver.resolve_installable(input)?,
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
