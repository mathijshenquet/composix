//! Experimental offline import of Docker archives and OCI image layouts.

pub mod cli;

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;

#[derive(Debug)]
pub struct ImportResult {
    pub store_path: PathBuf,
    pub service_name: String,
    pub findings: Vec<String>,
}

#[derive(Debug)]
pub struct AssembledImage {
    pub service_name: String,
    pub findings: Vec<String>,
}

#[derive(Debug)]
struct ImageSource {
    config: PathBuf,
    layers: Vec<PathBuf>,
    suggested_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DockerArchiveEntry {
    config: String,
    #[serde(default)]
    repo_tags: Vec<String>,
    layers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OciIndex {
    manifests: Vec<OciDescriptor>,
}

#[derive(Debug, Deserialize)]
struct OciManifest {
    config: OciDescriptor,
    layers: Vec<OciDescriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OciDescriptor {
    #[serde(default)]
    media_type: String,
    digest: String,
    #[serde(default)]
    annotations: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ImageConfigEnvelope {
    config: Option<ImageConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ImageConfig {
    #[serde(default)]
    env: Vec<String>,
    entrypoint: Option<Vec<String>>,
    cmd: Option<Vec<String>>,
    #[serde(default)]
    exposed_ports: BTreeMap<String, Value>,
    #[serde(default)]
    volumes: BTreeMap<String, Value>,
    working_dir: Option<String>,
    user: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedSpec {
    cix_spec: u32,
    services: BTreeMap<String, GeneratedService>,
}

#[derive(Debug, Serialize)]
struct GeneratedService {
    exec: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, GeneratedEnv>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    ports: BTreeMap<String, GeneratedPort>,
    #[serde(skip_serializing_if = "GeneratedDirs::is_empty")]
    dirs: GeneratedDirs,
}

#[derive(Debug, Serialize)]
struct GeneratedEnv {
    default: String,
}

#[derive(Debug, Serialize)]
struct GeneratedPort {
    value: u16,
    protocol: String,
}

#[derive(Debug, Default, Serialize)]
struct GeneratedDirs {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    state: Vec<String>,
}

impl GeneratedDirs {
    fn is_empty(&self) -> bool {
        self.state.is_empty()
    }
}

pub fn import(input: &Path, requested_name: Option<&str>) -> Result<ImportResult> {
    let item = TempDir::new().context("failed to create temporary import directory")?;
    let assembled = assemble(input, item.path(), requested_name)?;
    let store_name = format!("cix-import-{}", assembled.service_name);
    let output = Command::new("nix")
        .args(["store", "add-path", "--name"])
        .arg(&store_name)
        .arg(item.path())
        .output()
        .context("could not execute nix store add-path")?;
    if !output.status.success() {
        bail!(
            "nix store add-path failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let store_path = PathBuf::from(
        String::from_utf8(output.stdout)
            .context("nix store add-path returned non-UTF-8 output")?
            .trim(),
    );
    if !store_path.starts_with("/nix/store") {
        bail!(
            "nix store add-path returned an unexpected path: {}",
            store_path.display()
        );
    }
    Ok(ImportResult {
        store_path,
        service_name: assembled.service_name,
        findings: assembled.findings,
    })
}

pub fn assemble(input: &Path, item: &Path, requested_name: Option<&str>) -> Result<AssembledImage> {
    if item.exists() && item.read_dir()?.next().is_some() {
        bail!("import destination {} is not empty", item.display());
    }
    fs::create_dir_all(item.join("rootfs"))
        .with_context(|| format!("failed to create {}", item.join("rootfs").display()))?;

    let docker_archive = if input.is_file() {
        let unpacked =
            TempDir::new().context("failed to create Docker archive staging directory")?;
        unpack_archive(input, unpacked.path())
            .with_context(|| format!("failed to unpack Docker archive {}", input.display()))?;
        Some(unpacked)
    } else {
        None
    };
    let source_root = docker_archive
        .as_ref()
        .map_or(input, |directory| directory.path());
    let source = if docker_archive.is_some() {
        docker_source(source_root)?
    } else if input.is_dir() {
        oci_source(source_root)?
    } else {
        bail!(
            "input {} must be a docker-archive tarball or OCI layout directory",
            input.display()
        );
    };

    for (index, layer) in source.layers.iter().enumerate() {
        apply_layer(layer, &item.join("rootfs")).with_context(|| {
            format!("failed to apply layer {} ({})", index + 1, layer.display())
        })?;
    }

    let config_bytes = fs::read(&source.config)
        .with_context(|| format!("failed to read image config {}", source.config.display()))?;
    let envelope: ImageConfigEnvelope =
        serde_json::from_slice(&config_bytes).context("failed to parse image config JSON")?;
    let config = envelope.config.unwrap_or_default();
    let service_name = sanitize_name(
        requested_name
            .map(ToOwned::to_owned)
            .or(source.suggested_name)
            .as_deref()
            .unwrap_or("imported"),
    )?;
    let (spec, findings) = generate_spec(&config, &service_name, &item.join("rootfs"))?;
    let spec_bytes =
        serde_json::to_vec_pretty(&spec).context("failed to serialize generated cix spec")?;
    fs::write(item.join("cix-spec.json"), spec_bytes)
        .context("failed to write generated cix-spec.json")?;

    Ok(AssembledImage {
        service_name,
        findings,
    })
}

fn unpack_archive(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut archive = tar::Archive::new(BufReader::new(file));
    archive.unpack(destination)?;
    Ok(())
}

fn docker_source(root: &Path) -> Result<ImageSource> {
    let manifest_path = root.join("manifest.json");
    let manifest: Vec<DockerArchiveEntry> = serde_json::from_slice(
        &fs::read(&manifest_path)
            .with_context(|| format!("missing Docker manifest {}", manifest_path.display()))?,
    )
    .context("failed to parse Docker manifest.json")?;
    if manifest.len() != 1 {
        bail!(
            "Docker archive contains {} images; this prototype requires exactly one",
            manifest.len()
        );
    }
    let entry = manifest.into_iter().next().unwrap();
    let suggested_name = entry.repo_tags.first().map(|tag| image_name_from_ref(tag));
    Ok(ImageSource {
        config: checked_child(root, &entry.config)?,
        layers: entry
            .layers
            .iter()
            .map(|layer| checked_child(root, layer))
            .collect::<Result<_>>()?,
        suggested_name,
    })
}

fn oci_source(root: &Path) -> Result<ImageSource> {
    let layout = root.join("oci-layout");
    if !layout.is_file() {
        bail!(
            "directory {} is not an OCI layout (missing oci-layout)",
            root.display()
        );
    }
    let index_path = root.join("index.json");
    let index: OciIndex = serde_json::from_slice(
        &fs::read(&index_path)
            .with_context(|| format!("missing OCI index {}", index_path.display()))?,
    )
    .context("failed to parse OCI index.json")?;
    if index.manifests.len() != 1 {
        bail!(
            "OCI layout index contains {} manifests; this prototype requires exactly one",
            index.manifests.len()
        );
    }
    let descriptor = index.manifests.into_iter().next().unwrap();
    if descriptor.media_type.contains("image.index")
        || descriptor.media_type.contains("manifest.list")
    {
        bail!("nested/multi-platform OCI indexes are not supported by this prototype");
    }
    let manifest_path = blob_path(root, &descriptor.digest)?;
    let manifest: OciManifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .with_context(|| format!("failed to read OCI manifest {}", manifest_path.display()))?,
    )
    .context("failed to parse OCI image manifest")?;
    let suggested_name = descriptor
        .annotations
        .get("org.opencontainers.image.ref.name")
        .map(|name| image_name_from_ref(name));
    Ok(ImageSource {
        config: blob_path(root, &manifest.config.digest)?,
        layers: manifest
            .layers
            .iter()
            .map(|layer| {
                if layer.media_type.contains("+zstd") {
                    bail!("zstd-compressed OCI layers are not supported by this prototype");
                }
                blob_path(root, &layer.digest)
            })
            .collect::<Result<_>>()?,
        suggested_name,
    })
}

fn blob_path(root: &Path, digest: &str) -> Result<PathBuf> {
    let (algorithm, hash) = digest
        .split_once(':')
        .with_context(|| format!("invalid OCI digest {digest:?}"))?;
    if algorithm != "sha256"
        || hash.len() != 64
        || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("unsupported or malformed OCI digest {digest:?}");
    }
    Ok(root.join("blobs").join(algorithm).join(hash))
}

fn checked_child(root: &Path, child: &str) -> Result<PathBuf> {
    let path = Path::new(child);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        bail!("archive manifest contains unsafe path {child:?}");
    }
    Ok(root.join(path))
}

fn apply_layer(layer: &Path, rootfs: &Path) -> Result<()> {
    let whiteouts = scan_whiteouts(layer)?;
    for whiteout in whiteouts {
        match whiteout {
            Whiteout::Remove(path) => remove_path(&rootfs.join(path))?,
            Whiteout::Opaque(directory) => clear_directory(&rootfs.join(directory))?,
        }
    }

    let reader = layer_reader(layer)?;
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = clean_tar_path(&entry.path()?)?;
        if whiteout_for_path(&path)?.is_some() {
            continue;
        }
        entry
            .unpack_in(rootfs)
            .with_context(|| format!("failed to unpack layer entry {}", path.display()))?;
    }
    Ok(())
}

#[derive(Debug)]
enum Whiteout {
    Remove(PathBuf),
    Opaque(PathBuf),
}

fn scan_whiteouts(layer: &Path) -> Result<Vec<Whiteout>> {
    let reader = layer_reader(layer)?;
    let mut archive = tar::Archive::new(reader);
    let mut whiteouts = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        let path = clean_tar_path(&entry.path()?)?;
        if let Some(whiteout) = whiteout_for_path(&path)? {
            whiteouts.push(whiteout);
        }
    }
    Ok(whiteouts)
}

fn whiteout_for_path(path: &Path) -> Result<Option<Whiteout>> {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return Ok(None);
    };
    let Some(parent) = path.parent() else {
        return Ok(None);
    };
    if name == ".wh..wh..opq" {
        return Ok(Some(Whiteout::Opaque(parent.to_owned())));
    }
    if let Some(target) = name.strip_prefix(".wh.") {
        if target.is_empty() {
            bail!("invalid empty whiteout target in {}", path.display());
        }
        return Ok(Some(Whiteout::Remove(parent.join(target))));
    }
    Ok(None)
}

fn layer_reader(layer: &Path) -> Result<Box<dyn Read>> {
    let mut file = File::open(layer)
        .with_context(|| format!("failed to open layer archive {}", layer.display()))?;
    let mut magic = [0; 2];
    let count = file.read(&mut magic)?;
    drop(file);
    let file = File::open(layer)?;
    if count == 2 && magic == [0x1f, 0x8b] {
        Ok(Box::new(GzDecoder::new(BufReader::new(file))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

fn clean_tar_path(path: &Path) -> Result<PathBuf> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("layer contains unsafe path {}", path.display())
            }
        }
    }
    if clean.as_os_str().is_empty() {
        bail!("layer contains an empty path");
    }
    Ok(clean)
}

fn remove_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn clear_directory(path: &Path) -> Result<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        remove_path(&entry?.path())?;
    }
    Ok(())
}

fn generate_spec(
    config: &ImageConfig,
    service_name: &str,
    rootfs: &Path,
) -> Result<(GeneratedSpec, Vec<String>)> {
    let mut findings = Vec::new();
    let env = config
        .env
        .iter()
        .filter_map(|assignment| assignment.split_once('='))
        .map(|(name, value)| {
            (
                name.to_owned(),
                GeneratedEnv {
                    default: value.to_owned(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut exec = config.entrypoint.clone().unwrap_or_default();
    exec.extend(config.cmd.clone().unwrap_or_default());
    if exec.is_empty() {
        bail!("image config has neither Entrypoint nor Cmd");
    }
    exec[0] = rootfs_relative_executable(&exec[0], &env, rootfs);

    let mut ports = BTreeMap::new();
    for declaration in config.exposed_ports.keys() {
        let (number, protocol) = declaration
            .split_once('/')
            .unwrap_or((declaration.as_str(), "tcp"));
        let Ok(value) = number.parse::<u16>() else {
            findings.push(format!(
                "ExposedPorts entry {declaration:?} is not a numeric port and was skipped"
            ));
            continue;
        };
        if value == 0 || !matches!(protocol, "tcp" | "udp") {
            findings.push(format!(
                "ExposedPorts entry {declaration:?} is outside cix's tcp/udp port model and was skipped"
            ));
            continue;
        }
        ports.insert(
            format!("port-{value}-{protocol}"),
            GeneratedPort {
                value,
                protocol: protocol.to_owned(),
            },
        );
    }

    let dirs = GeneratedDirs {
        state: config.volumes.keys().cloned().collect(),
    };
    for volume in &dirs.state {
        if !is_v2_state_path(volume) {
            findings.push(format!(
                "Docker volume {volume:?} was recorded in dirs.state as requested, but cix-spec v2 only permits one component below /var/lib; today's runner will reject this generated spec"
            ));
        }
    }
    if let Some(working_dir) = config
        .working_dir
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        findings.push(format!(
            "image WorkingDir={working_dir:?} cannot be expressed by cix-spec v2"
        ));
    }
    if let Some(user) = config.user.as_deref().filter(|value| !value.is_empty()) {
        findings.push(format!(
            "image User={user:?} cannot be expressed by cix-spec v2"
        ));
    }

    let mut services = BTreeMap::new();
    services.insert(
        service_name.to_owned(),
        GeneratedService {
            exec,
            env,
            ports,
            dirs,
        },
    );
    Ok((
        GeneratedSpec {
            cix_spec: 2,
            services,
        },
        findings,
    ))
}

fn rootfs_relative_executable(
    executable: &str,
    env: &BTreeMap<String, GeneratedEnv>,
    rootfs: &Path,
) -> String {
    if executable.starts_with('/') {
        return format!("rootfs/{}", executable.trim_start_matches('/'));
    }
    if executable.contains('/') {
        return format!("rootfs/{}", executable.trim_start_matches("./"));
    }
    let path = env
        .get("PATH")
        .map(|value| value.default.as_str())
        .unwrap_or("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    for directory in path
        .split(':')
        .filter(|directory| directory.starts_with('/'))
    {
        let candidate = Path::new(directory).join(executable);
        if rootfs.join(candidate.strip_prefix("/").unwrap()).is_file() {
            return format!("rootfs{}", candidate.display());
        }
    }
    format!("rootfs/{executable}")
}

fn is_v2_state_path(path: &str) -> bool {
    let Ok(relative) = Path::new(path).strip_prefix("/var/lib") else {
        return false;
    };
    let mut components = relative.components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn image_name_from_ref(reference: &str) -> String {
    let without_digest = reference.split('@').next().unwrap_or(reference);
    let last = without_digest.rsplit('/').next().unwrap_or(without_digest);
    last.rsplit_once(':')
        .map_or(last, |(name, _)| name)
        .to_owned()
}

fn sanitize_name(name: &str) -> Result<String> {
    let sanitized = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(['-', '.'])
        .to_owned();
    if sanitized.is_empty()
        || !sanitized
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        bail!("could not derive a valid service name from {name:?}");
    }
    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn imports_docker_archive_and_applies_whiteouts() {
        let fixture = TempDir::new().unwrap();
        let lower = fixture.path().join("lower.tar");
        tar_file(
            &lower,
            &[
                ("etc/keep", b"old"),
                ("etc/remove", b"gone"),
                ("var/cache/old", b"stale"),
            ],
        );
        let upper = fixture.path().join("upper.tar");
        tar_file(
            &upper,
            &[
                ("etc/.wh.remove", b""),
                ("etc/keep", b"new"),
                ("var/cache/.wh..wh..opq", b""),
                ("var/cache/fresh", b"fresh"),
            ],
        );
        let config_name = "config.json";
        fs::write(
            fixture.path().join(config_name),
            r#"{"config":{"Env":["PORT=8080","A=a=b"],"Entrypoint":["/bin/app"],"Cmd":["serve"],"ExposedPorts":{"8080/tcp":{}},"Volumes":{"/data":{}},"WorkingDir":"/work","User":"1000"}}"#,
        )
        .unwrap();
        fs::write(
            fixture.path().join("manifest.json"),
            format!(
                r#"[{{"Config":"{config_name}","RepoTags":["example.test/team/demo:v1"],"Layers":["lower.tar","upper.tar"]}}]"#
            ),
        )
        .unwrap();
        let docker_tar = fixture.path().join("docker.tar");
        tar_directory(
            &docker_tar,
            fixture.path(),
            &["manifest.json", config_name, "lower.tar", "upper.tar"],
        );

        let output = TempDir::new().unwrap();
        let assembled = assemble(&docker_tar, output.path(), None).unwrap();
        assert_eq!(assembled.service_name, "demo");
        assert_eq!(
            fs::read(output.path().join("rootfs/etc/keep")).unwrap(),
            b"new"
        );
        assert!(!output.path().join("rootfs/etc/remove").exists());
        assert!(!output.path().join("rootfs/var/cache/old").exists());
        assert_eq!(
            fs::read(output.path().join("rootfs/var/cache/fresh")).unwrap(),
            b"fresh"
        );
        let spec: Value =
            serde_json::from_slice(&fs::read(output.path().join("cix-spec.json")).unwrap())
                .unwrap();
        let service = &spec["services"]["demo"];
        assert_eq!(
            service["exec"],
            serde_json::json!(["rootfs/bin/app", "serve"])
        );
        assert_eq!(service["env"]["A"]["default"], "a=b");
        assert_eq!(service["ports"]["port-8080-tcp"]["value"], 8080);
        assert_eq!(service["dirs"]["state"], serde_json::json!(["/data"]));
        assert_eq!(assembled.findings.len(), 3);
    }

    #[test]
    fn imports_gzip_layer_from_oci_layout() {
        let fixture = TempDir::new().unwrap();
        fs::create_dir_all(fixture.path().join("blobs/sha256")).unwrap();
        fs::write(
            fixture.path().join("oci-layout"),
            r#"{"imageLayoutVersion":"1.0.0"}"#,
        )
        .unwrap();

        let layer_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let layer = fixture.path().join("blobs/sha256").join(layer_hash);
        let raw_layer = fixture.path().join("raw-layer.tar");
        tar_file(&raw_layer, &[("usr/local/bin/demo", b"binary")]);
        let mut encoder =
            flate2::write::GzEncoder::new(File::create(&layer).unwrap(), Default::default());
        encoder.write_all(&fs::read(raw_layer).unwrap()).unwrap();
        encoder.finish().unwrap();

        let config_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        fs::write(
            fixture.path().join("blobs/sha256").join(config_hash),
            r#"{"config":{"Env":["PATH=/usr/local/bin"],"Entrypoint":["demo"]}}"#,
        )
        .unwrap();
        let manifest_hash = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        fs::write(
            fixture.path().join("blobs/sha256").join(manifest_hash),
            format!(
                r#"{{"schemaVersion":2,"config":{{"digest":"sha256:{config_hash}"}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:{layer_hash}"}}]}}"#
            ),
        )
        .unwrap();
        fs::write(
            fixture.path().join("index.json"),
            format!(
                r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{manifest_hash}","annotations":{{"org.opencontainers.image.ref.name":"demo:v1"}}}}]}}"#
            ),
        )
        .unwrap();

        let output = TempDir::new().unwrap();
        assemble(fixture.path(), output.path(), None).unwrap();
        let spec: Value =
            serde_json::from_slice(&fs::read(output.path().join("cix-spec.json")).unwrap())
                .unwrap();
        assert_eq!(
            spec["services"]["demo"]["exec"],
            serde_json::json!(["rootfs/usr/local/bin/demo"])
        );
    }

    #[test]
    fn rejects_archive_path_traversal() {
        assert!(clean_tar_path(Path::new("../escape")).is_err());
        assert!(checked_child(Path::new("/tmp/root"), "../escape").is_err());
    }

    fn tar_file(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut builder = tar::Builder::new(file);
        for (name, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_path(name).unwrap();
            header.set_size(contents.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append(&header, *contents).unwrap();
        }
        builder.finish().unwrap();
    }

    fn tar_directory(path: &Path, root: &Path, names: &[&str]) {
        let file = File::create(path).unwrap();
        let mut builder = tar::Builder::new(file);
        for name in names {
            builder
                .append_path_with_name(root.join(name), name)
                .unwrap();
        }
        builder.finish().unwrap();
    }
}
