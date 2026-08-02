//! FETCH credentials and consent state.
//!
//! [`ConsentStore`] owns the persisted approval boundary; [`HostCredentials`]
//! exposes only command-scoped credential mounts to the build conductor.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialsFile {
    #[serde(default)]
    tokens: BTreeMap<String, CredentialToken>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialToken {
    pub(crate) url: String,
    pub(crate) credential: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Consent {
    pub(crate) project: PathBuf,
    pub(crate) token: String,
    pub(crate) prefix: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsentStore {
    #[serde(default)]
    pub(crate) grants: BTreeSet<Consent>,
}

pub(crate) struct CredentialMount {
    pub(crate) name: String,
    pub(crate) source: PathBuf,
}

pub(crate) struct HostCredentials {
    pub(crate) project: PathBuf,
    pub(crate) tokens: BTreeMap<String, CredentialToken>,
    pub(crate) consent_path: PathBuf,
    pub(crate) consent: ConsentStore,
    pub(crate) allow_secret: bool,
}

impl HostCredentials {
    pub(crate) fn load(project: &Path, allow_secret: bool) -> Result<Self> {
        let project = project
            .canonicalize()
            .context("canonicalizing FETCH credential project")?;
        let config_path = credential_config_path()?;
        let tokens = match fs::read(&config_path) {
            Ok(bytes) => {
                serde_json::from_slice::<CredentialsFile>(&bytes)
                    .with_context(|| {
                        format!("parsing FETCH credentials file {}", config_path.display())
                    })?
                    .tokens
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error).context("reading FETCH credentials file"),
        };
        let consent_path = consent_store_path()?;
        let consent = match fs::read(&consent_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("parsing FETCH consent store")?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => ConsentStore::default(),
            Err(error) => return Err(error).context("reading FETCH consent store"),
        };
        Ok(Self {
            project,
            tokens,
            consent_path,
            consent,
            allow_secret,
        })
    }

    pub(crate) fn for_command(&mut self, command: &str) -> Result<Option<CredentialMount>> {
        let Some(url) = concrete_fetch_url(command) else {
            return Ok(None);
        };
        let prefix = url_prefix(&url)?;
        let matches = self
            .tokens
            .iter()
            .filter(|(_, token)| token_matches(&token.url, &url))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let existing = self
            .consent
            .grants
            .iter()
            .find(|grant| grant.project == self.project && grant.prefix == prefix)
            .cloned();
        if matches.is_empty() {
            if let Some(grant) = existing {
                bail!("FETCH of {url} needs previously approved token {}; that token is no longer configured (refusing anonymous retry)", grant.token);
            }
            return Ok(None);
        }
        let name = if let Some(grant) = existing {
            if !matches.contains(&grant.token) {
                bail!("FETCH of {url} needs previously approved token {}; that token is no longer configured for this URL (refusing anonymous retry)", grant.token);
            }
            grant.token
        } else if matches.len() == 1 {
            matches[0].clone()
        } else {
            choose_token(&url, &matches, self.allow_secret)?
        };
        let grant = Consent {
            project: self.project.clone(),
            token: name.clone(),
            prefix,
        };
        if !self.consent.grants.contains(&grant) && !self.allow_secret {
            eprint!("allow FETCH of {url} using {name}? y/N ");
            io::stderr().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
                bail!("FETCH credential use was not approved");
            }
            self.consent.grants.insert(grant);
            self.save()?;
        }
        let token = &self.tokens[&name];
        if !token.credential.is_file() {
            bail!("FETCH token {name} has no readable credential file (refusing anonymous retry)");
        }
        Ok(Some(CredentialMount {
            name,
            source: token.credential.clone(),
        }))
    }

    fn save(&self) -> Result<()> {
        let parent = self
            .consent_path
            .parent()
            .expect("consent state path has a parent");
        fs::create_dir_all(parent).context("creating FETCH consent state directory")?;
        let temporary =
            tempfile::NamedTempFile::new_in(parent).context("creating FETCH consent state")?;
        serde_json::to_writer_pretty(temporary.reopen()?, &self.consent)?;
        temporary
            .persist(&self.consent_path)
            .map_err(|error| error.error)
            .context("saving FETCH consent state")?;
        Ok(())
    }
}

pub fn revoke_fetch_consent(token: &str) -> Result<usize> {
    let path = consent_store_path()?;
    let mut store: ConsentStore = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("parsing FETCH consent store")?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error).context("reading FETCH consent store"),
    };
    let removed = revoke_from_store(&mut store, token);
    if removed != 0 {
        let parent = path.parent().expect("consent state path has a parent");
        let temporary =
            tempfile::NamedTempFile::new_in(parent).context("creating FETCH consent state")?;
        serde_json::to_writer_pretty(temporary.reopen()?, &store)?;
        temporary
            .persist(path)
            .map_err(|error| error.error)
            .context("saving FETCH consent state")?;
    }
    Ok(removed)
}

pub(crate) fn revoke_from_store(store: &mut ConsentStore, token: &str) -> usize {
    let before = store.grants.len();
    store.grants.retain(|grant| grant.token != token);
    before - store.grants.len()
}

fn credential_config_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("CREDENTIALS_DIRECTORY") {
        return Ok(PathBuf::from(directory).join("credentials"));
    }
    let home = std::env::var_os("HOME")
        .context("HOME is unset; set CREDENTIALS_DIRECTORY for FETCH credentials")?;
    Ok(PathBuf::from(home).join(".config/cix/credentials"))
}

fn consent_store_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(directory).join("cix/fetch-consents.json"));
    }
    let home = std::env::var_os("HOME")
        .context("HOME is unset; set XDG_STATE_HOME for FETCH consent state")?;
    Ok(PathBuf::from(home).join(".local/state/cix/fetch-consents.json"))
}

pub(crate) fn concrete_fetch_url(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '\"']))
        .find(|word| word.starts_with("https://") || word.starts_with("http://"))
        .map(ToOwned::to_owned)
}

pub(crate) fn url_prefix(url: &str) -> Result<String> {
    let (_, after_scheme) = url
        .split_once("://")
        .context("FETCH URL must have a scheme")?;
    let (host, path) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    let first = path.split('/').next().filter(|part| !part.is_empty());
    Ok(match first {
        Some(first) => format!(
            "{}://{host}/{first}",
            &url[..url.find("://").expect("scheme exists")]
        ),
        None => format!(
            "{}://{host}",
            &url[..url.find("://").expect("scheme exists")]
        ),
    })
}

pub(crate) fn token_matches(pattern: &str, url: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => url.starts_with(prefix) && url.ends_with(suffix),
        None => url.starts_with(pattern),
    }
}

fn choose_token(url: &str, matches: &[String], allow_secret: bool) -> Result<String> {
    if allow_secret {
        bail!(
            "FETCH of {url} matches multiple credentials ({}); --allow-secret cannot choose one",
            matches.join(", ")
        );
    }
    eprint!(
        "FETCH of {url} matches credentials {}; choose token name: ",
        matches.join(", ")
    );
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    if matches.iter().any(|name| name == answer) {
        Ok(answer.to_owned())
    } else {
        bail!("no matching FETCH credential was selected")
    }
}
