//! HTTP index serving and cache synchronization.

use super::refs::*;
use anyhow::{anyhow, Context, Result};
use cix_common::{current_system, nix, Ref};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

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

fn closure_size_text(store_path: &str) -> Option<String> {
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

fn manifest_summary(store_path: &str) -> Option<String> {
    let contents = fs::read(Path::new(store_path).join("cix-manifest.json")).ok()?;
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
            summary = output.and_then(|output| manifest_summary(&output.store_path));
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
            .and_then(|output| closure_size_text(&output.store_path))
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
    store: &Store,
    listen: &str,
    substituters: Vec<String>,
    with_store: bool,
    sign_key: Option<&str>,
) -> Result<()> {
    let cache = store.root().join("store");
    if with_store {
        sync_cache(store, &cache, sign_key)?;
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
            sync_cache(store, &cache, sign_key)?;
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
                let has_tag = input
                    .rsplit_once('/')
                    .map_or(input, |(_, last)| last)
                    .contains(':');
                let parsed = if has_tag {
                    Ref::parse(input)
                } else {
                    Ref::parse(&format!("{input}:tag"))
                }
                .ok()
                .filter(|reference| reference.root_url.is_none());
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
