//! Shared types and nix interop for composix.
//!
//! Ownership: the index track owns ref parsing and the local tag store; the
//! run track owns spec types. Genuinely shared pieces (store path handling,
//! `nix` subprocess helpers) live here.
//!
//! ## Module map
//!
//! Intentional module-map omission: this crate has only its cfg-gated `tests`
//! module, so its production code remains in this small root.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

// Process-wide instrumentation deliberately shares this counter across Nix helpers.
static NIX_SUBPROCESS_COUNT: AtomicU64 = AtomicU64::new(0);

// Signal handlers can only communicate with foreground loops through an atomic flag.
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub fn reset_nix_subprocess_count() {
    NIX_SUBPROCESS_COUNT.store(0, Ordering::Relaxed);
}

pub fn nix_subprocess_count() -> u64 {
    NIX_SUBPROCESS_COUNT.load(Ordering::Relaxed)
}

pub fn record_nix_subprocess() {
    NIX_SUBPROCESS_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// A Docker-shaped cix name. `root_url` is an identity, rather than a socket.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ref {
    pub root_url: Option<String>,
    pub name: String,
    pub tag: String,
}

impl Ref {
    /// Parses `[root_url/]name:tag` using Docker's registry disambiguation.
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() || input.starts_with('/') || input.ends_with('/') {
            bail!("invalid ref `{input}`");
        }

        let slash = input.rfind('/');
        let tag_start = input.rfind(':').filter(|colon| match slash {
            Some(last_slash) => *colon > last_slash,
            None => true,
        });
        let Some(colon) = tag_start else {
            bail!(
                "ref `{input}` has no explicit tag; :latest is not a thing here — write {input}:<tag>"
            );
        };
        let (without_tag, tag) = (&input[..colon], &input[colon + 1..]);
        if !valid_part(tag) {
            bail!("invalid tag `{tag}` in ref `{input}`");
        }

        let mut components = without_tag.split('/');
        let first = components.next().expect("input was checked nonempty");
        if first.is_empty() {
            bail!("invalid ref `{input}`");
        }
        let has_port = first.rsplit_once(':').is_some_and(|(_, port)| {
            !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
        });
        let has_root = first == "localhost" || first.contains('.') || has_port;
        let (root_url, name) = if has_root {
            let name = components.collect::<Vec<_>>().join("/");
            (Some(first.to_owned()), name)
        } else {
            (None, without_tag.to_owned())
        };
        let name_parts = name.split('/').collect::<Vec<_>>();
        if name.is_empty()
            || name_parts.len() > 2
            || !name_parts.iter().all(|part| valid_part(part))
        {
            bail!("invalid name `{name}` in ref `{input}`");
        }
        Ok(Self {
            root_url,
            name,
            tag: tag.to_owned(),
        })
    }

    pub fn display(&self) -> String {
        let prefix = self
            .root_url
            .as_ref()
            .map(|root| format!("{root}/"))
            .unwrap_or_default();
        format!("{prefix}{}:{}", self.name, self.tag)
    }

    pub fn looks_like_untagged_ref(input: &str) -> bool {
        !input.contains('#')
            && !input.starts_with('/')
            && !input.contains("://")
            && Self::parse(&format!("{input}:tag")).is_ok()
    }
}

fn valid_part(part: &str) -> bool {
    !part.is_empty()
        && part.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

/// Runs `nix`, returning stdout and carrying both stderr and command context on failure.
pub fn nix(args: &[&str]) -> Result<String> {
    record_nix_subprocess();
    let output = Command::new("nix")
        .args(args)
        .output()
        .with_context(|| format!("could not execute nix {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "nix {} failed ({}): {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("nix returned non-UTF-8 stdout")
}

pub fn current_system() -> Result<String> {
    let architecture = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        architecture => {
            bail!("unsupported host architecture {architecture:?} for Nix system detection")
        }
    };
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        os => bail!("unsupported host OS {os:?} for Nix system detection"),
    };
    Ok(format!("{architecture}-{os}"))
}

pub fn build_installable(installable: &str) -> Result<String> {
    let output = nix(&["build", "--no-link", "--print-out-paths", installable])?;
    output
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| format!("nix build did not print an output path for `{installable}`"))
}

#[cfg(test)]
mod tests {
    use super::Ref;

    #[test]
    fn local_ref_requires_an_explicit_tag() {
        assert_eq!(
            Ref::parse("team/my_app:v1").unwrap(),
            Ref {
                root_url: None,
                name: "team/my_app".into(),
                tag: "v1".into(),
            }
        );
    }

    #[test]
    fn registry_disambiguation() {
        for (input, root, name, tag) in [
            ("localhost/family/x:v1", "localhost", "family/x", "v1"),
            ("localhost:8420/a/b:v1", "localhost:8420", "a/b", "v1"),
            (
                "cix.example.com/team/app:stable",
                "cix.example.com",
                "team/app",
                "stable",
            ),
            ("127.0.0.1/family/x:1", "127.0.0.1", "family/x", "1"),
        ] {
            let parsed = Ref::parse(input).unwrap();
            assert_eq!(parsed.root_url.as_deref(), Some(root));
            assert_eq!(parsed.name, name);
            assert_eq!(parsed.tag, tag);
        }
    }

    #[test]
    fn colons_before_name_are_ports_not_tags() {
        let parsed = Ref::parse("localhost:8420/family/demo:v1").unwrap();
        assert_eq!(parsed.root_url.as_deref(), Some("localhost:8420"));
        assert_eq!(parsed.tag, "v1");
    }

    #[test]
    fn missing_tag_explains_that_latest_does_not_exist() {
        let error = Ref::parse("family/member").unwrap_err().to_string();
        assert!(error.contains(":latest is not a thing here"), "{error}");
    }

    #[test]
    fn recognizes_docker_shaped_untagged_names() {
        assert!(Ref::looks_like_untagged_ref("family/member"));
        assert!(Ref::looks_like_untagged_ref(
            "cix.example.com/family/member"
        ));
        assert!(!Ref::looks_like_untagged_ref(".#package"));
        assert!(!Ref::looks_like_untagged_ref("family/member:v1"));
    }

    #[test]
    fn rejects_nasty_refs() {
        for input in [
            "", "/x", "x/", "x", "Upper", "x:bad!", "a//b", "a:b/c", "a/b/c:v1",
        ] {
            assert!(Ref::parse(input).is_err(), "{input} should fail");
        }
    }
}
