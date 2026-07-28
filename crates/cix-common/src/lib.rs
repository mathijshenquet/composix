//! Shared types and nix interop for composix.
//!
//! Ownership: the index track owns ref parsing and the local tag store; the
//! run track owns spec types. Genuinely shared pieces (store path handling,
//! `nix` subprocess helpers) live here.

use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// A Docker-shaped cix name. `root_url` is an identity, rather than a socket.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ref {
    pub root_url: Option<String>,
    pub name: String,
    pub tag: String,
}

impl Ref {
    /// Parses `[root_url/]name[:tag]` using Docker's registry disambiguation.
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() || input.starts_with('/') || input.ends_with('/') {
            bail!("invalid ref `{input}`");
        }

        let slash = input.rfind('/');
        let tag_start = input.rfind(':').filter(|colon| match slash {
            Some(last_slash) => *colon > last_slash,
            None => true,
        });
        let (without_tag, tag) = match tag_start {
            Some(colon) => (&input[..colon], &input[colon + 1..]),
            None => (input, "latest"),
        };
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
        if name.is_empty() || !name.split('/').all(valid_part) {
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
}

fn valid_part(part: &str) -> bool {
    !part.is_empty()
        && part.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

/// Runs `nix`, returning stdout and carrying both stderr and command context on failure.
pub fn nix(args: &[&str]) -> Result<String> {
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
    Ok(nix(&[
        "eval",
        "--impure",
        "--raw",
        "--expr",
        "builtins.currentSystem",
    ])?
    .trim()
    .to_owned())
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
    fn local_ref_and_default_tag() {
        assert_eq!(
            Ref::parse("team/my_app").unwrap(),
            Ref {
                root_url: None,
                name: "team/my_app".into(),
                tag: "latest".into(),
            }
        );
    }

    #[test]
    fn registry_disambiguation() {
        for (input, root, name, tag) in [
            ("localhost/x:v1", "localhost", "x", "v1"),
            ("localhost:8420/a/b", "localhost:8420", "a/b", "latest"),
            (
                "cix.example.com/team/app:stable",
                "cix.example.com",
                "team/app",
                "stable",
            ),
            ("127.0.0.1/x:1", "127.0.0.1", "x", "1"),
        ] {
            let parsed = Ref::parse(input).unwrap();
            assert_eq!(parsed.root_url.as_deref(), Some(root));
            assert_eq!(parsed.name, name);
            assert_eq!(parsed.tag, tag);
        }
    }

    #[test]
    fn colons_before_name_are_ports_not_tags() {
        let parsed = Ref::parse("localhost:8420/demo").unwrap();
        assert_eq!(parsed.root_url.as_deref(), Some("localhost:8420"));
        assert_eq!(parsed.tag, "latest");
    }

    #[test]
    fn rejects_nasty_refs() {
        for input in [
            "",
            "/x",
            "x/",
            "Upper",
            "x:bad!",
            "localhost",
            "a//b",
            "a:b/c",
        ] {
            assert!(Ref::parse(input).is_err(), "{input} should fail");
        }
    }
}
