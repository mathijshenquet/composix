//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See docs/design.md "Part 1 — index".

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
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

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
    if reference.root_url.is_some() && upstream.is_none() {
        bail!(
            "qualified names denote remote state; tags are bare. To publish, tag on the box that serves (see docs/design.md \"The org workflow\")."
        );
    }
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

fn negotiated_response(
    body: Vec<u8>,
    status: StatusCode,
    content_type: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", content_type).expect("valid header"))
        .with_header(Header::from_bytes("Vary", "Accept").expect("valid header"))
}

fn json_response<T: Serialize>(
    value: &T,
    status: StatusCode,
) -> Response<std::io::Cursor<Vec<u8>>> {
    negotiated_response(
        serde_json::to_vec(value).expect("serializing API response"),
        status,
        "application/vnd.cix+json;version=1",
    )
}

fn html_response(body: String, status: StatusCode) -> Response<std::io::Cursor<Vec<u8>>> {
    negotiated_response(body.into_bytes(), status, "text/html; charset=utf-8")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn html_document(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font:16px system-ui,sans-serif;max-width:70rem;margin:2rem auto;padding:0 1rem}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #bbb;padding:.4rem;text-align:left;vertical-align:top}}code{{background:#f3f3f3;padding:.15rem .3rem}}</style></head><body>{body}</body></html>",
        html_escape(title)
    )
}

fn age(created_at: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    created_at
        .parse::<u64>()
        .map(|created| format!("{}s", now.saturating_sub(created)))
        .unwrap_or_else(|_| "unknown".into())
}

fn closure_size(store_path: &str) -> Option<String> {
    nix(&["path-info", "-S", store_path])
        .ok()
        .and_then(|output| {
            output
                .split_whitespace()
                .last()
                .filter(|size| size.bytes().all(|byte| byte.is_ascii_digit()))
                .map(ToOwned::to_owned)
        })
}

fn spec_summary(store_path: &str) -> Option<String> {
    let contents = fs::read(Path::new(store_path).join("cix-spec.json")).ok()?;
    let spec: serde_json::Value = serde_json::from_slice(&contents).ok()?;
    let services = spec.get("services")?.as_object()?;
    let mut rows = String::new();
    for (service_name, service) in services {
        let service = service.as_object()?;
        let ports = service
            .get("ports")
            .and_then(serde_json::Value::as_object)
            .map(|ports| ports.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let environment = service
            .get("env")
            .and_then(serde_json::Value::as_object)
            .map(|environment| environment.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(service_name),
            html_escape(&ports),
            html_escape(&environment)
        ));
    }
    Some(format!(
        "<section><h2>Spec summary</h2><table><thead><tr><th>Service</th><th>Ports</th><th>Environment</th></tr></thead><tbody>{rows}</tbody></table></section>"
    ))
}

fn header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|candidate| candidate.field.equiv(name))
        .map(|candidate| candidate.value.as_str())
}

fn request_origin(request: &Request) -> (String, String) {
    let host = header(request, "Host").unwrap_or("localhost").to_owned();
    let scheme = header(request, "X-Forwarded-Proto")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("http")
        .to_owned();
    (host, scheme)
}

fn wants_json(request: &Request, query: Option<&str>) -> bool {
    let format = query.and_then(|query| {
        query.split('&').find_map(|parameter| {
            parameter
                .split_once('=')
                .and_then(|(key, value)| (key == "format").then_some(value))
        })
    });
    match format {
        Some("json") => true,
        Some("html") => false,
        _ => header(request, "Accept").is_some_and(|accept| {
            accept.split(',').any(|media_range| {
                let media_type = media_range.split(';').next().unwrap_or_default().trim();
                media_type.eq_ignore_ascii_case("application/vnd.cix+json")
                    || media_type.eq_ignore_ascii_case("application/json")
            })
        }),
    }
}

fn bare_tags(tags: Vec<TagMetadata>) -> Vec<(Ref, TagMetadata)> {
    tags.into_iter()
        .filter_map(|metadata| {
            Ref::parse(&metadata.reference)
                .ok()
                .map(|reference| (reference, metadata))
        })
        .filter(|(reference, _)| reference.root_url.is_none())
        .collect()
}

fn names_page(tags: &[(Ref, TagMetadata)]) -> String {
    let names = tags
        .iter()
        .map(|(reference, _)| reference.name.clone())
        .collect::<BTreeSet<_>>();
    let links = names
        .iter()
        .map(|name| {
            format!(
                "<li><a href=\"/{}\">{}</a></li>",
                html_escape(name),
                html_escape(name)
            )
        })
        .collect::<String>();
    html_document("cix index", format!("<h1>cix index</h1><ul>{links}</ul>"))
}

fn name_page(host: &str, name: &str, tags: &[(Ref, TagMetadata)]) -> String {
    let system = current_system().ok();
    let mut rows = String::new();
    let mut summary = None;
    for (reference, metadata) in tags {
        let output = system
            .as_deref()
            .and_then(|system| metadata.entry.outputs.get(system));
        if summary.is_none() {
            summary = output.and_then(|output| spec_summary(&output.store_path));
        }
        let systems = metadata
            .entry
            .outputs
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let store_path = output
            .map(|output| output.store_path.as_str())
            .unwrap_or("-");
        let nar_hash = output.map(|output| output.nar_hash.as_str()).unwrap_or("-");
        let size = output
            .and_then(|output| closure_size(&output.store_path))
            .unwrap_or_else(|| "unknown".into());
        let permalink = format!("/{}:{}", reference.name, reference.tag);
        rows.push_str(&format!(
            "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&permalink),
            html_escape(&reference.tag),
            html_escape(&systems),
            html_escape(store_path),
            html_escape(nar_hash),
            html_escape(&size),
            html_escape(&age(&metadata.entry.created_at)),
        ));
    }
    let pull_snippets = tags
        .iter()
        .map(|(reference, _)| {
            format!(
                "<p><code>cix pull {}/{}</code></p>",
                html_escape(host),
                html_escape(&format!("{}:{}", name, reference.tag))
            )
        })
        .collect::<String>();
    html_document(
        &format!("{host}/{name}"),
        format!(
            "<h1>{}/{}</h1><table><thead><tr><th>Tag</th><th>Systems</th><th>Store path</th><th>narHash</th><th>Closure size</th><th>Age</th></tr></thead><tbody>{rows}</tbody></table>{pull_snippets}{}",
            html_escape(host),
            html_escape(name),
            summary.unwrap_or_default(),
        ),
    )
}

fn detail_page(host: &str, reference: &Ref, metadata: &TagMetadata) -> String {
    let mut rows = String::new();
    for (system, output) in &metadata.entry.outputs {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(system),
            html_escape(&output.store_path),
            html_escape(&output.nar_hash),
            html_escape(output.drv_path.as_deref().unwrap_or("-")),
        ));
    }
    let full_ref = format!("{host}/{}:{}", reference.name, reference.tag);
    html_document(
        &full_ref,
        format!(
            "<h1>{}</h1><table><thead><tr><th>System</th><th>Store path</th><th>narHash</th><th>drvPath</th></tr></thead><tbody>{rows}</tbody></table><p>createdAt: {}</p><p><code>cix pull {}</code></p><p><code>cix run {}</code></p>",
            html_escape(&full_ref),
            html_escape(&metadata.entry.created_at),
            html_escape(&full_ref),
            html_escape(&full_ref),
        ),
    )
}

fn not_found_html() -> String {
    html_document("Not found", "<h1>Not found</h1>".into())
}

fn copy_to_cache(cache: &Path, output: &Output, sign_key: Option<&str>) -> Result<()> {
    if let Some(key) = sign_key {
        nix(&["store", "sign", "--key-file", key, &output.store_path])?;
    }
    let cache_url = format!("file://{}", cache.display());
    nix(&["copy", "--to", &cache_url, &output.store_path])?;
    Ok(())
}

fn sync_cache(store: &Store, cache: &Path, sign_key: Option<&str>) -> Result<()> {
    if cache.exists() {
        fs::remove_dir_all(cache)
            .with_context(|| format!("rebuilding cix binary cache {}", cache.display()))?;
    }
    fs::create_dir_all(cache)?;
    for metadata in store.all()? {
        let reference = Ref::parse(&metadata.reference)?;
        if reference.root_url.is_none() {
            for output in metadata.entry.outputs.values() {
                copy_to_cache(cache, output, sign_key)?;
            }
        }
    }
    Ok(())
}

/// Serves indefinitely. The request loop reloads sidecars so a long-running
/// server notices tags created after it was started.
pub fn serve(
    listen: &str,
    substituters: Vec<String>,
    with_store: bool,
    sign_key: Option<&str>,
) -> Result<()> {
    let store = Store::open()?;
    let cache = store.root().join("store");
    if with_store {
        sync_cache(&store, &cache, sign_key)?;
    }
    let server = Server::http(listen).map_err(|error| anyhow!(error))?;
    eprintln!("cix index listening on {listen}");
    for request in server.incoming_requests() {
        if request.method() != &Method::Get {
            request.respond(Response::empty(StatusCode(405)))?;
            continue;
        }
        let (url, query) = request.url().split_once('?').unwrap_or((request.url(), ""));
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
            sync_cache(&store, &cache, sign_key)?;
        }
        let tags = bare_tags(tags);
        let (host, scheme) = request_origin(&request);
        let mut advertised = substituters.clone();
        if with_store {
            advertised.push(format!("{scheme}://{host}/store"));
        }
        let json = wants_json(&request, (!query.is_empty()).then_some(query));
        let response = match url {
            "/" => {
                let names = tags
                    .iter()
                    .map(|(reference, _)| reference.name.clone())
                    .collect::<BTreeSet<_>>();
                if json {
                    json_response(&serde_json::json!({"names": names}), StatusCode(200))
                } else {
                    html_response(names_page(&tags), StatusCode(200))
                }
            }
            path => {
                let input = path.strip_prefix('/').unwrap_or_default();
                let parsed = Ref::parse(input)
                    .ok()
                    .filter(|reference| reference.root_url.is_none());
                let has_tag = input
                    .rsplit_once('/')
                    .map_or(input, |(_, last)| last)
                    .contains(':');
                match (parsed, has_tag) {
                    (Some(reference), true) => {
                        match tags.iter().find(|(candidate, _)| candidate == &reference) {
                            Some((_, metadata)) if json => {
                                json_response(&api_entry(metadata, &advertised), StatusCode(200))
                            }
                            Some((_, metadata)) => html_response(
                                detail_page(&host, &reference, metadata),
                                StatusCode(200),
                            ),
                            None if json => json_response(
                                &serde_json::json!({"error": "unknown tag"}),
                                StatusCode(404),
                            ),
                            None => html_response(not_found_html(), StatusCode(404)),
                        }
                    }
                    (Some(reference), false) => {
                        let matching = tags
                            .iter()
                            .filter(|(candidate, _)| candidate.name == reference.name)
                            .cloned()
                            .collect::<Vec<_>>();
                        if matching.is_empty() {
                            if json {
                                json_response(
                                    &serde_json::json!({"error": "unknown name"}),
                                    StatusCode(404),
                                )
                            } else {
                                html_response(not_found_html(), StatusCode(404))
                            }
                        } else if json {
                            let entries = matching
                                .iter()
                                .map(|(reference, metadata)| {
                                    (reference.tag.clone(), api_entry(metadata, &advertised))
                                })
                                .collect::<BTreeMap<_, _>>();
                            json_response(&serde_json::json!({"tags": entries}), StatusCode(200))
                        } else {
                            html_response(
                                name_page(&host, &reference.name, &matching),
                                StatusCode(200),
                            )
                        }
                    }
                    _ if json => {
                        json_response(&serde_json::json!({"error": "not found"}), StatusCode(404))
                    }
                    _ => html_response(not_found_html(), StatusCode(404)),
                }
            }
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
    let url = endpoint(reference, &format!("/{}:{}", reference.name, reference.tag))?;
    let response = ureq::get(&url)
        .set("Accept", "application/vnd.cix+json;version=1")
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
