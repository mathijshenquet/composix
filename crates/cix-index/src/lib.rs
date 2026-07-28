//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See DESIGN.md "Part 1 — index".

pub mod cli;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use cix_common::{build_installable, current_system, nix, Ref};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Response, Server, StatusCode};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub store_path: String,
    pub nar_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drv_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub outputs: BTreeMap<String, Output>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub substituters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_keys: Vec<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagMetadata {
    pub reference: String,
    #[serde(flatten)]
    pub entry: Entry,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathInfo {
    nar_hash: String,
    #[serde(default)]
    deriver: Option<String>,
}

/// The on-disk user index. Base64-url encoding is injective, filesystem-safe,
/// and avoids `/` and `:` from refs becoming accidental directories or names.
#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open() -> Result<Self> {
        let root = match env::var_os("CIX_STATE_DIR") {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(env::var_os("HOME").context("HOME is unset; set CIX_STATE_DIR")?)
                .join(".local/state/cix"),
        };
        let store = Self { root };
        fs::create_dir_all(store.roots_dir()).context("creating cix roots directory")?;
        fs::create_dir_all(store.meta_dir()).context("creating cix metadata directory")?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn roots_dir(&self) -> PathBuf {
        self.root.join("roots")
    }

    fn meta_dir(&self) -> PathBuf {
        self.root.join("tags")
    }

    pub fn encode(reference: &Ref) -> String {
        URL_SAFE_NO_PAD.encode(reference.display())
    }

    fn link_path(&self, reference: &Ref) -> PathBuf {
        self.roots_dir().join(Self::encode(reference))
    }

    fn metadata_path(&self, reference: &Ref) -> PathBuf {
        self.meta_dir()
            .join(format!("{}.json", Self::encode(reference)))
    }

    pub fn load(&self, reference: &Ref) -> Result<Option<TagMetadata>> {
        let path = self.metadata_path(reference);
        match fs::read(&path) {
            Ok(contents) => Ok(Some(
                serde_json::from_slice(&contents)
                    .with_context(|| format!("parsing tag sidecar {}", path.display()))?,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("reading tag sidecar {}", path.display()))
            }
        }
    }

    pub fn all(&self) -> Result<Vec<TagMetadata>> {
        let mut tags: Vec<TagMetadata> = Vec::new();
        for item in fs::read_dir(self.meta_dir()).context("listing tag sidecars")? {
            let item = item?;
            if item
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("json")
            {
                continue;
            }
            tags.push(serde_json::from_slice(&fs::read(item.path())?)?);
        }
        tags.sort_by(|left, right| left.reference.cmp(&right.reference));
        Ok(tags)
    }

    pub fn save(&self, reference: &Ref, metadata: &TagMetadata) -> Result<()> {
        let path = self.metadata_path(reference);
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(metadata)?)
            .with_context(|| format!("writing tag sidecar {}", temporary.display()))?;
        fs::rename(temporary, path).context("atomically replacing tag sidecar")?;
        Ok(())
    }

    pub fn remove(&self, reference: &Ref) -> Result<bool> {
        let link = self.link_path(reference);
        let metadata = self.metadata_path(reference);
        let mut existed = false;
        for path in [link, metadata] {
            match fs::remove_file(&path) {
                Ok(()) => existed = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).with_context(|| format!("removing {}", path.display()))
                }
            }
        }
        Ok(existed)
    }

    pub fn register_root(&self, reference: &Ref, store_path: &str) -> Result<()> {
        let link = self.link_path(reference);
        if link.exists() || fs::symlink_metadata(&link).is_ok() {
            fs::remove_file(&link)
                .with_context(|| format!("replacing GC root {}", link.display()))?;
        }
        let link_text = link.to_string_lossy().into_owned();
        nix(&["build", store_path, "--out-link", &link_text])?;
        Ok(())
    }
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs()
        .to_string()
}

fn path_info(store_path: &str) -> Result<Output> {
    let raw = nix(&["path-info", "--json", "--json-format", "1", store_path])?;
    let infos: BTreeMap<String, PathInfo> =
        serde_json::from_str(&raw).context("parsing nix path-info JSON")?;
    let (path, info) = infos
        .into_iter()
        .next()
        .context("nix path-info returned no paths")?;
    Ok(Output {
        store_path: path,
        nar_hash: info.nar_hash,
        drv_path: info.deriver,
    })
}

fn resolved_path(store: &Store, installable: &str) -> Result<String> {
    if installable.starts_with("/nix/store/") {
        return Ok(installable.to_owned());
    }
    if let Ok(reference) = Ref::parse(installable) {
        if let Some(metadata) = store.load(&reference)? {
            let system = current_system()?;
            return metadata
                .entry
                .outputs
                .get(&system)
                .map(|output| output.store_path.clone())
                .with_context(|| format!("alias `{installable}` has no output for {system}"));
        }
    }
    build_installable(installable)
}

pub fn tag(installable: &str, target: &str, upstream: Option<String>) -> Result<()> {
    let store = Store::open()?;
    let reference = Ref::parse(target)?;
    let path = resolved_path(&store, installable)?;
    let output = path_info(&path)?;
    let system = current_system()?;
    let mut metadata = store.load(&reference)?.unwrap_or_else(|| TagMetadata {
        reference: reference.display(),
        entry: Entry {
            outputs: BTreeMap::new(),
            substituters: Vec::new(),
            trusted_keys: Vec::new(),
            created_at: timestamp(),
        },
        upstream: upstream.clone(),
    });
    metadata.reference = reference.display();
    metadata.entry.outputs.insert(system, output.clone());
    if metadata.upstream.is_none() {
        metadata.upstream = upstream;
    }
    store.register_root(&reference, &output.store_path)?;
    store.save(&reference, &metadata)
}

pub fn untag(target: &str) -> Result<()> {
    let store = Store::open()?;
    let reference = Ref::parse(target)?;
    if !store.remove(&reference)? {
        bail!("tag `{}` does not exist", reference.display());
    }
    Ok(())
}

pub fn list(prefix: Option<&str>, long: bool) -> Result<String> {
    let store = Store::open()?;
    let system = current_system()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let lines = store
        .all()?
        .into_iter()
        .filter(|tag| prefix.is_none_or(|prefix| tag.reference.starts_with(prefix)))
        .map(|tag| {
            if !long {
                return tag.reference;
            }
            let systems = tag
                .entry
                .outputs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(",");
            let path = tag
                .entry
                .outputs
                .get(&system)
                .map(|output| output.store_path.as_str())
                .unwrap_or("-");
            let age = tag
                .entry
                .created_at
                .parse::<u64>()
                .ok()
                .map(|created| format!("{}s", now.saturating_sub(created)))
                .unwrap_or_else(|| "unknown".into());
            format!(
                "{}\tsystems={}\tpath={}\tupstream={}\tage={}",
                tag.reference,
                systems,
                path,
                tag.upstream.unwrap_or_else(|| "-".into()),
                age
            )
        })
        .collect::<Vec<_>>();
    Ok(lines.join("\n"))
}

fn api_entry(metadata: &TagMetadata, substituters: &[String]) -> Entry {
    let mut entry = metadata.entry.clone();
    entry.substituters = substituters.to_vec();
    entry
}

fn json_response<T: Serialize>(
    value: &T,
    status: StatusCode,
) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(serde_json::to_vec(value).expect("serializing API response"))
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").expect("valid header"))
}

fn copy_to_cache(cache: &Path, output: &Output, sign_key: Option<&str>) -> Result<()> {
    if let Some(key) = sign_key {
        nix(&["store", "sign", "--key-file", key, &output.store_path])?;
    }
    let cache_url = format!("file://{}", cache.display());
    nix(&["copy", "--to", &cache_url, &output.store_path])?;
    Ok(())
}

/// Serves indefinitely. The request loop reloads sidecars so a long-running
/// server notices tags created after it was started.
pub fn serve(
    root_url: &str,
    listen: &str,
    substituters: Vec<String>,
    with_store: bool,
    sign_key: Option<&str>,
) -> Result<()> {
    let store = Store::open()?;
    let cache = store.root().join("store");
    if with_store {
        fs::create_dir_all(&cache)?;
        for metadata in store.all()? {
            let reference = Ref::parse(&metadata.reference)?;
            if reference.root_url.as_deref() == Some(root_url) {
                for output in metadata.entry.outputs.values() {
                    copy_to_cache(&cache, output, sign_key)?;
                }
            }
        }
    }
    let mut advertised = substituters;
    if with_store {
        advertised.push(format!("http://{listen}/store"));
    }
    let server = Server::http(listen).map_err(|error| anyhow!(error))?;
    eprintln!("cix index for {root_url} listening on {listen}");
    for request in server.incoming_requests() {
        if request.method() != &Method::Get {
            request.respond(Response::empty(StatusCode(405)))?;
            continue;
        }
        let url = request.url().split('?').next().unwrap_or(request.url());
        if with_store && url.starts_with("/store/") {
            let relative = &url["/store/".len()..];
            if relative.contains("..") || relative.is_empty() {
                request.respond(Response::empty(StatusCode(404)))?;
                continue;
            }
            let file = cache.join(relative);
            match fs::File::open(&file) {
                Ok(mut handle) => {
                    let mut bytes = Vec::new();
                    handle.read_to_end(&mut bytes)?;
                    request.respond(Response::from_data(bytes))?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    request.respond(Response::empty(StatusCode(404)))?;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("reading cache file {}", file.display()))
                }
            }
            continue;
        }

        let tags = store.all()?;
        if with_store {
            for metadata in &tags {
                let reference = Ref::parse(&metadata.reference)?;
                if reference.root_url.as_deref() == Some(root_url) {
                    for output in metadata.entry.outputs.values() {
                        copy_to_cache(&cache, output, sign_key)?;
                    }
                }
            }
        }
        let response = if let Some(rest) = url.strip_prefix("/v1/resolve/") {
            let mut parts = rest.rsplitn(2, '/');
            let tag = parts.next().unwrap_or_default();
            let name = parts.next().unwrap_or_default();
            let sought = Ref {
                root_url: Some(root_url.to_owned()),
                name: name.to_owned(),
                tag: tag.to_owned(),
            }
            .display();
            match tags.iter().find(|metadata| metadata.reference == sought) {
                Some(metadata) => json_response(&api_entry(metadata, &advertised), StatusCode(200)),
                None => json_response(
                    &serde_json::json!({"error": "unknown tag"}),
                    StatusCode(404),
                ),
            }
        } else if let Some(name) = url.strip_prefix("/v1/tags/") {
            let prefix = format!("{root_url}/{name}:");
            let entries = tags
                .iter()
                .filter(|metadata| metadata.reference.starts_with(&prefix))
                .filter_map(|metadata| {
                    Ref::parse(&metadata.reference)
                        .ok()
                        .map(|reference| (reference.tag, api_entry(metadata, &advertised)))
                })
                .collect::<BTreeMap<_, _>>();
            if entries.is_empty() {
                json_response(
                    &serde_json::json!({"error": "unknown name"}),
                    StatusCode(404),
                )
            } else {
                json_response(&serde_json::json!({"tags": entries}), StatusCode(200))
            }
        } else if url == "/v1/names" {
            let names = tags
                .iter()
                .filter_map(|metadata| Ref::parse(&metadata.reference).ok())
                .filter(|reference| reference.root_url.as_deref() == Some(root_url))
                .map(|reference| reference.name)
                .collect::<BTreeSet<_>>();
            json_response(&serde_json::json!({"names": names}), StatusCode(200))
        } else {
            json_response(&serde_json::json!({"error": "not found"}), StatusCode(404))
        };
        request.respond(response)?;
    }
    Ok(())
}

fn endpoint(reference: &Ref, path: &str) -> Result<String> {
    let root = reference
        .root_url
        .as_deref()
        .context("pull requires a remote root_url")?;
    let scheme = if root.starts_with("localhost") || root.starts_with("127.") {
        "http"
    } else {
        "https"
    };
    Ok(format!("{scheme}://{root}{path}"))
}

fn resolve_remote(reference: &Ref) -> Result<Entry> {
    let url = endpoint(
        reference,
        &format!("/v1/resolve/{}/{}", reference.name, reference.tag),
    )?;
    let response = ureq::get(&url)
        .call()
        .map_err(|error| anyhow!("resolving {url}: {error}"))?;
    if response.status() != 200 {
        bail!("resolving {url} returned HTTP {}", response.status());
    }
    response
        .into_json()
        .context("parsing index resolve response")
}

fn pull_one(remote: &Ref, local: &Ref) -> Result<bool> {
    let entry = resolve_remote(remote)?;
    let system = current_system()?;
    let output = entry
        .outputs
        .get(&system)
        .with_context(|| format!("remote `{}` has no output for {system}", remote.display()))?;
    let store = Store::open()?;
    if store
        .load(local)?
        .and_then(|metadata| metadata.entry.outputs.get(&system).cloned())
        .is_some_and(|existing| existing.nar_hash == output.nar_hash)
    {
        return Ok(false);
    }
    if entry.substituters.is_empty() {
        bail!(
            "remote `{}` did not advertise a substituter",
            remote.display()
        );
    }
    let mut copied = false;
    let mut failures = Vec::new();
    for substituter in &entry.substituters {
        let trusted_keys = entry.trusted_keys.join(" ");
        let mut arguments = vec!["copy", "--from", substituter.as_str()];
        if !trusted_keys.is_empty() {
            arguments.extend(["--option", "trusted-public-keys", &trusted_keys]);
        }
        arguments.push(&output.store_path);
        match nix(&arguments) {
            Ok(_) => {
                copied = true;
                break;
            }
            Err(error) => failures.push(format!("{substituter}: {error:#}")),
        }
    }
    if !copied {
        bail!(
            "could not fetch {} from any substituter: {}",
            output.store_path,
            failures.join("; ")
        );
    }
    let actual = path_info(&output.store_path)?;
    if actual.nar_hash != output.nar_hash {
        bail!(
            "narHash mismatch for {}: index has {}, local store has {}",
            output.store_path,
            output.nar_hash,
            actual.nar_hash
        );
    }
    tag(
        &output.store_path,
        &local.display(),
        remote.root_url.clone(),
    )?;
    Ok(true)
}

pub fn pull(reference: Option<&str>, as_ref: Option<&str>) -> Result<usize> {
    match reference {
        Some(input) => {
            let remote = Ref::parse(input)?;
            if remote.root_url.is_none() {
                bail!("pull requires a fully-qualified ref with a root_url");
            }
            let local = match as_ref {
                Some(alias) => Ref::parse(alias)?,
                None => remote.clone(),
            };
            Ok(usize::from(pull_one(&remote, &local)?))
        }
        None => {
            if as_ref.is_some() {
                bail!("--as requires a remote ref");
            }
            let store = Store::open()?;
            let mut changed = 0;
            for metadata in store.all()? {
                let Some(upstream) = metadata.upstream else {
                    continue;
                };
                let local = Ref::parse(&metadata.reference)?;
                let mut remote = local.clone();
                remote.root_url = Some(upstream);
                changed += usize::from(pull_one(&remote, &local)?);
            }
            Ok(changed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, Output, Store, TagMetadata};
    use cix_common::Ref;
    use std::{collections::BTreeMap, fs};

    #[test]
    fn encoding_is_safe_and_distinct() {
        let left = Ref::parse("cix.example.com/team/app:v1").unwrap();
        let right = Ref::parse("cix.example.com/team/app:v2").unwrap();
        assert_ne!(Store::encode(&left), Store::encode(&right));
        assert!(Store::encode(&left)
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
    }

    #[test]
    fn sidecar_round_trip() {
        let root = std::env::temp_dir().join(format!("cix-index-test-{}", std::process::id()));
        let store = Store { root: root.clone() };
        fs::create_dir_all(store.meta_dir()).unwrap();
        let reference = Ref::parse("localhost:8420/x:v1").unwrap();
        let metadata = TagMetadata {
            reference: reference.display(),
            entry: Entry {
                outputs: BTreeMap::from([(
                    "x86_64-linux".into(),
                    Output {
                        store_path: "/nix/store/example".into(),
                        nar_hash: "sha256-test".into(),
                        drv_path: None,
                    },
                )]),
                substituters: vec![],
                trusted_keys: vec![],
                created_at: "1".into(),
            },
            upstream: Some("localhost:8420".into()),
        };
        store.save(&reference, &metadata).unwrap();
        assert_eq!(store.load(&reference).unwrap(), Some(metadata));
        fs::remove_dir_all(root).unwrap();
    }
}
