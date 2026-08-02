//! Device claim property assembly.
//!
//! This module owns the service device policy; the unit conductor decides
//! where that policy appears among the hardening properties.

use std::collections::BTreeSet;
use std::ffi::CStr;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use anyhow::{Context, Result};

use crate::spec::Service;

pub(crate) fn add_policy(properties: &mut Vec<(String, String)>, service: &Service) -> Result<()> {
    if !service.has_device_claim() {
        properties.push(("PrivateDevices".into(), "yes".into()));
        return Ok(());
    }

    properties.push(("DevicePolicy".into(), "closed".into()));
    let mut groups = BTreeSet::new();
    if service.has_claim("gpu") {
        properties.push(("DeviceAllow".into(), "/dev/dri rwm".into()));
        groups.extend(["render".to_owned(), "video".to_owned()]);
    }
    for device in service.device_claims() {
        properties.push(("DeviceAllow".into(), format!("{} rwm", device.display())));
        if let Some(group) = device_group(device)? {
            groups.insert(group);
        }
    }
    if !groups.is_empty() {
        properties.push((
            "SupplementaryGroups".into(),
            groups.into_iter().collect::<Vec<_>>().join(" "),
        ));
    }
    Ok(())
}

fn device_group(device: &Path) -> Result<Option<String>> {
    let metadata = match std::fs::metadata(device) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "warning: claimed device {} is absent while generating the unit; no owning group was added and activation will fail until the hardware is present",
                device.display()
            );
            return Ok(None);
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("statting claimed device {}", device.display()))
        }
    };
    let group = unsafe { libc::getgrgid(metadata.gid()) };
    if group.is_null() {
        eprintln!(
            "warning: claimed device {} has gid {} with no resolvable group; no supplementary group was added",
            device.display(),
            metadata.gid()
        );
        return Ok(None);
    }
    let name = unsafe { CStr::from_ptr((*group).gr_name) }
        .to_str()
        .context("claimed device group name is not valid UTF-8")?;
    Ok((!name.is_empty()).then(|| name.to_owned()))
}
