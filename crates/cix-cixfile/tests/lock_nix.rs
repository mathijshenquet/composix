use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use cix_build::ArtifactResolver;
use cix_cixfile::generate_nix_with_snapshots;
use cix_cixfile::{
    build, build_family, build_family_with_stats_file, build_with_registry, generate_nix, parse,
    ArtifactPin, ArtifactRegistry, BuildOptions, LockFile,
};

struct TestRegistry(cix_index::Store);

impl ArtifactResolver for TestRegistry {
    fn resolve_artifact(&self, reference: &str) -> anyhow::Result<ArtifactPin> {
        let output = cix_index::resolve_with(&self.0, reference).with_context(|| {
            format!("resolving cix-item FROM ref {reference:?}; pull it or tag it first")
        })?;
        Ok(ArtifactPin {
            store_path: output.store_path,
            nar_hash: output.nar_hash,
        })
    }
}

impl ArtifactRegistry for TestRegistry {
    fn tag_artifact(&self, store_path: &str, reference: &str) -> anyhow::Result<()> {
        cix_index::tag(&self.0, store_path, reference, None)
    }
}

fn test_workspace_directory() -> PathBuf {
    tempfile::tempdir().unwrap().keep()
}

fn committed_lock() -> LockFile {
    let input = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../examples/pack/nginx/Cixfile.lock"
        ))
        .unwrap()["inputs"]["pkgs"]
            .clone(),
    )
    .unwrap();
    LockFile {
        inputs: std::collections::BTreeMap::from([("pkgs".into(), input)]),
        artifacts: std::collections::BTreeMap::new(),
        fetches: std::collections::BTreeMap::new(),
        memo: std::collections::BTreeMap::new(),
        step_memo: std::collections::BTreeMap::new(),
        dev_envs: std::collections::BTreeMap::new(),
        builder_dev_envs: std::collections::BTreeMap::new(),
        eval_plan: None,
        outputs: std::collections::BTreeMap::new(),
    }
}

#[test]
fn named_cixfiles_build_independently_with_sibling_locks() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("default.txt"), "default\n").unwrap();
    fs::write(directory.path().join("dissolved.txt"), "dissolved\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nITEM default\nCOPY default.txt /result\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.dissolved"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nITEM dissolved\nCOPY dissolved.txt /result\n",
    )
    .unwrap();
    for lock_name in ["Cixfile.lock", "Cixfile.dissolved.lock"] {
        fs::write(
            directory.path().join(lock_name),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&committed_lock()).unwrap()
            ),
        )
        .unwrap();
    }
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    };

    let default = build(&options).unwrap();
    let dissolved = build_family_with_stats_file(&options, &[], None, None, "Cixfile.dissolved")
        .unwrap()
        .0;
    assert_eq!(
        fs::read_to_string(Path::new(&default[0].store_path).join("result")).unwrap(),
        "default\n"
    );
    assert_eq!(
        fs::read_to_string(Path::new(&dissolved[0].store_path).join("result")).unwrap(),
        "dissolved\n"
    );
    let default_lock = fs::read_to_string(directory.path().join("Cixfile.lock")).unwrap();
    let dissolved_lock =
        fs::read_to_string(directory.path().join("Cixfile.dissolved.lock")).unwrap();
    assert!(default_lock.contains("default"));
    assert!(!default_lock.contains("dissolved"));
    assert!(dissolved_lock.contains("dissolved"));
    assert!(!dissolved_lock.contains("default"));
}

#[test]
fn with_spec_redis_builds_mounts_and_parses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().unwrap();
    let expression = format!(
        r#"let pkgs = import (builtins.getFlake "path:{}").inputs.nixpkgs {{ system = "x86_64-linux"; }}; in import {}/examples/pack/redis {{ inherit pkgs; }}"#,
        root.display(),
        root.display(),
    );
    let output = build_expression(&expression).unwrap();

    assert!(output.join("etc/redis/redis.conf").is_file());
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    assert_eq!(spec.cix_manifest, 0);
    assert_eq!(
        spec.select_service(None)
            .unwrap()
            .1
            .mounts
            .as_deref()
            .unwrap(),
        [PathBuf::from("/etc/redis")]
    );
}

fn build_expression(expression: &str) -> anyhow::Result<PathBuf> {
    let output = cix_common::nix(&[
        "build",
        "--impure",
        "--no-link",
        "--print-out-paths",
        "--expr",
        expression,
    ])?;
    Ok(PathBuf::from(output.trim()))
}

#[test]
fn fhs_glibc_and_musl_elfs_run_from_loader_aliases_without_cixfile_fixups() {
    let fixture = fhs_elf_fixture();
    let directory = tempfile::tempdir().unwrap();
    fs::copy(
        fixture.join("fhs-probe"),
        directory.path().join("fhs-probe"),
    )
    .unwrap();
    fs::copy(
        fixture.join("musl-fhs-probe"),
        directory.path().join("musl-fhs-probe"),
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER gnu
IMPORT ${pkgs.bash} ${pkgs.glibc}
COPY ${src}/fhs-probe .
RUN ./fhs-probe > gnu-result
BUILDER musl
IMPORT ${pkgs.bash} ${pkgs.musl}
COPY ${src}/musl-fhs-probe .
RUN ./musl-fhs-probe > musl-result
ITEM gnu-result
COPY ${gnu}/gnu-result /result
ITEM musl-result
COPY ${musl}/musl-result /result
"#,
    )
    .unwrap();
    write_committed_lock(directory.path());
    let built = build(&fhs_build_options(directory.path())).unwrap();

    for item in &built {
        assert_eq!(
            fs::read_to_string(Path::new(&item.store_path).join("result")).unwrap(),
            "fhs-alias-ok\n",
            "{}",
            item.name
        );
    }
    let cixfile = fs::read_to_string(directory.path().join("Cixfile")).unwrap();
    assert!(!cixfile.contains("patchelf"));
}

fn fhs_elf_fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().unwrap();
    build_expression(&format!(
        r#"let pkgs = import (builtins.getFlake "path:{}").inputs.nixpkgs {{ system = "x86_64-linux"; }}; in
pkgs.runCommand "cix-fhs-elf" {{ nativeBuildInputs = [ pkgs.gcc pkgs.patchelf ]; }} ''
  printf '#include <stdio.h>\nint main(void) {{ puts("fhs-alias-ok"); return 0; }}\n' > probe.c
  cc probe.c -o probe
  patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 --remove-rpath probe
  ${{pkgs.pkgsMusl.stdenv.cc}}/bin/cc probe.c -o musl-probe
  patchelf --set-interpreter /lib/ld-musl-x86_64.so.1 --remove-rpath musl-probe
  printf 'int cix_extra(void) {{ return 95; }}\n' > extra.c
  cc -fPIC -shared -Wl,-soname,libcix-extra.so.1 extra.c -o libcix-extra.so.1
  printf '#include <stdio.h>\nint cix_extra(void);\nint main(void) {{ printf("%d\\n", cix_extra()); return 0; }}\n' > needed.c
  cc needed.c -L. -Wl,-l:libcix-extra.so.1 -o needed-probe
  patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 --remove-rpath needed-probe
  mkdir -p "$out"
  cp probe "$out/fhs-probe"
  cp musl-probe "$out/musl-fhs-probe"
  cp needed-probe libcix-extra.so.1 "$out/"
''"#,
        root.display()
    ))
    .unwrap()
}

fn write_committed_lock(directory: &Path) {
    fs::write(
        directory.join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
}

fn fhs_build_options(directory: &Path) -> BuildOptions {
    BuildOptions {
        directory: directory.to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    }
}

#[test]
fn missing_fhs_loader_diagnostic_suggests_the_libc_import() {
    let fixture = fhs_elf_fixture();
    let directory = tempfile::tempdir().unwrap();
    fs::copy(
        fixture.join("fhs-probe"),
        directory.path().join("fhs-probe"),
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER missing
IMPORT ${pkgs.bash}
COPY ${src}/fhs-probe .
RUN ./fhs-probe
ITEM unreachable
COPY ${src}/fhs-probe /unreachable
"#,
    )
    .unwrap();
    write_committed_lock(directory.path());
    let error = format!(
        "{:#}",
        build(&fhs_build_options(directory.path())).unwrap_err()
    );
    assert!(
        error.contains("fhs-probe requires the FHS loader"),
        "{error}"
    );
    assert!(error.contains("/lib64/ld-linux-x86-64.so.2"), "{error}");
    assert!(error.contains("libc.so.6"), "{error}");
    assert!(error.contains("IMPORT ${pkgs.glibc}"), "{error}");
}

#[test]
fn beyond_libc_diagnostic_names_the_alias_boundary_and_patchelf_escape() {
    let fixture = fhs_elf_fixture();
    let directory = tempfile::tempdir().unwrap();
    for name in ["needed-probe", "libcix-extra.so.1"] {
        fs::copy(fixture.join(name), directory.path().join(name)).unwrap();
    }
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER beyond
IMPORT ${pkgs.bash} ${pkgs.glibc}
COPY ${src}/ .
RUN ./needed-probe
ITEM unreachable
COPY ${src}/needed-probe /unreachable
"#,
    )
    .unwrap();
    write_committed_lock(directory.path());
    let error = format!(
        "{:#}",
        build(&fhs_build_options(directory.path())).unwrap_err()
    );
    assert!(error.contains("libraries beyond that libc"), "{error}");
    assert!(error.contains("libcix-extra.so.1"), "{error}");
    assert!(error.contains("does not add a /lib search path"), "{error}");
    assert!(error.contains("IMPORT ${pkgs.patchelf}"), "{error}");
    assert!(
        error.contains("docs/migrate.md#fhs-linked-native-binaries"),
        "{error}"
    );
}

fn add_store_path(path: &Path) -> PathBuf {
    let path = path.to_str().expect("temporary path is UTF-8");
    let output = cix_common::nix(&["store", "add-path", path]).unwrap();
    PathBuf::from(output.trim())
}

#[test]
fn store_aware_copy_spike_covers_tomcat_directus_and_realpath() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("seed"), "builder placeholder\n").unwrap();
    fs::write(source.path().join("patch"), "tomcat patch\n").unwrap();
    fs::write(source.path().join("marker"), "force materialization\n").unwrap();

    let snapshot = tempfile::tempdir().unwrap();
    fs::create_dir_all(snapshot.path().join("tomcat/conf")).unwrap();
    fs::write(snapshot.path().join("tomcat/conf/base"), "tomcat base\n").unwrap();
    fs::create_dir_all(snapshot.path().join("dist/lib")).unwrap();
    fs::write(snapshot.path().join("dist/lib/server.js"), "directus\n").unwrap();
    fs::create_dir_all(snapshot.path().join("tree/lib")).unwrap();
    fs::write(snapshot.path().join("tree/lib/probe.js"), "probe\n").unwrap();
    let snapshot = add_store_path(snapshot.path());
    let snapshots = std::collections::BTreeMap::from([(
        "build".to_owned(),
        snapshot.to_string_lossy().into_owned(),
    )]);

    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
COPY seed seed
ITEM tomcat
COPY ${build}/tomcat /tomcat
COPY patch /tomcat/conf/patch
SERVICE directus
COPY ${build}/dist /app
STATEDIR /app/database
START ${pkgs.coreutils}/bin/true
ITEM linked-realpath
COPY ${build}/tree /app
ITEM materialized-realpath
COPY ${build}/tree /app
COPY marker /app/marker
"#,
    )
    .unwrap();
    let build = |name| {
        let expression = generate_nix_with_snapshots(
            &cixfile,
            name,
            source.path(),
            &committed_lock(),
            "x86_64-linux",
            &snapshots,
        )
        .unwrap();
        build_expression(&expression).unwrap()
    };

    let tomcat = build("tomcat");
    assert!(tomcat.join("tomcat").is_dir());
    assert_eq!(
        fs::read_to_string(tomcat.join("tomcat/conf/base")).unwrap(),
        "tomcat base\n"
    );
    assert_eq!(
        fs::read_to_string(tomcat.join("tomcat/conf/patch")).unwrap(),
        "tomcat patch\n"
    );

    let directus = build("directus");
    assert!(directus.join("app").is_dir());
    assert!(!fs::symlink_metadata(directus.join("app"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(directus.join("app/lib/server.js").is_file());

    let linked = build("linked-realpath");
    let materialized = build("materialized-realpath");
    assert!(fs::symlink_metadata(linked.join("app"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(materialized.join("app").is_dir());
    let linked_realpath = fs::canonicalize(linked.join("app/lib/probe.js")).unwrap();
    let materialized_realpath = fs::canonicalize(materialized.join("app/lib/probe.js")).unwrap();
    eprintln!("linked realpath: {}", linked_realpath.display());
    eprintln!("materialized realpath: {}", materialized_realpath.display());
    assert!(
        linked_realpath.starts_with("/nix/store"),
        "{linked_realpath:?}"
    );
    assert!(!linked_realpath.starts_with(&linked), "{linked_realpath:?}");
    assert!(
        materialized_realpath.starts_with(&materialized),
        "{materialized_realpath:?}"
    );
}

#[test]
fn artifact_import_unions_packages_for_services_apps_and_items() {
    let source = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for package in [first.path(), second.path()] {
        fs::create_dir_all(package.join("bin")).unwrap();
        fs::create_dir_all(package.join("etc/tool")).unwrap();
        fs::create_dir_all(package.join("share/tool")).unwrap();
    }
    fs::write(first.path().join("bin/collision"), "first\n").unwrap();
    fs::set_permissions(
        first.path().join("bin/collision"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(first.path().join("etc/tool/first"), "first\n").unwrap();
    fs::write(second.path().join("bin/collision"), "second\n").unwrap();
    fs::set_permissions(
        second.path().join("bin/collision"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(second.path().join("etc/tool/second"), "second\n").unwrap();
    fs::write(second.path().join("share/tool/data"), "shared\n").unwrap();
    let snapshots = std::collections::BTreeMap::from([
        (
            "first".to_owned(),
            add_store_path(first.path()).to_string_lossy().into_owned(),
        ),
        (
            "second".to_owned(),
            add_store_path(second.path()).to_string_lossy().into_owned(),
        ),
    ]);
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER first
BUILDER second
SERVICE union
IMPORT ${first} ${second}
START collision --flag
APP app
IMPORT ${pkgs.coreutils}
START true
ITEM item
IMPORT ${pkgs.coreutils}
"#,
    )
    .unwrap();
    let build = |name| {
        build_expression(
            &generate_nix_with_snapshots(
                &cixfile,
                name,
                source.path(),
                &committed_lock(),
                "x86_64-linux",
                &snapshots,
            )
            .unwrap(),
        )
        .unwrap()
    };

    let union = build("union");
    assert_eq!(
        fs::read_to_string(union.join("bin/collision")).unwrap(),
        "first\n"
    );
    assert_eq!(
        fs::read_to_string(union.join("etc/tool/first")).unwrap(),
        "first\n"
    );
    assert_eq!(
        fs::read_to_string(union.join("etc/tool/second")).unwrap(),
        "second\n"
    );
    assert_eq!(
        fs::read_to_string(union.join("share/tool/data")).unwrap(),
        "shared\n"
    );
    let spec = cix_run::spec::Spec::load(&union).unwrap();
    assert_eq!(
        spec.select_service(None).unwrap().1.start,
        ["bin/collision", "--flag"]
    );
    assert_eq!(
        spec.select_service(None)
            .unwrap()
            .1
            .mounts
            .as_deref()
            .unwrap(),
        [
            PathBuf::from("/bin/collision"),
            PathBuf::from("/etc/tool"),
            PathBuf::from("/share/tool")
        ]
    );

    let app = build("app");
    assert!(app.join("bin/true").is_file());
    assert_eq!(
        cix_run::spec::Spec::load(&app)
            .unwrap()
            .select_service(None)
            .unwrap()
            .1
            .start,
        ["bin/true"]
    );

    let item = build("item");
    assert!(item.join("bin/true").is_file());
    assert!(!item.join("cix-manifest.json").exists());
}

#[test]
fn nix_rejects_a_committed_lock_with_the_wrong_nar_hash() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nSTART /bin/fixture\n",
    )
    .unwrap();
    let mut lock = committed_lock();
    lock.inputs.get_mut("pkgs").unwrap().nar_hash =
        "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_owned();
    let expression = generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux").unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(
        error.contains("hash mismatch") || error.contains("does not match"),
        "{error}"
    );
}

#[test]
fn unknown_nixpkgs_attribute_includes_the_cixfile_line() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nCOPY ${pkgs.thisAttributeDoesNotExist}/bin/missing /bin/missing\nSTART /bin/missing\n",
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(error.contains("Cixfile line 3"), "{error}");
    assert!(error.contains("thisAttributeDoesNotExist"), "{error}");
}

#[test]
fn real_nix_build_assembles_files_links_and_spec() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
FILE /share/content <<EOF
package=${pkgs.hello}
escaped=$${literal}
runtime=$VALUE
EOF
COPY ${pkgs.hello}/bin/hello /bin/hello
START hello
"#,
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let output = build_expression(&expression).unwrap();

    let contents = fs::read_to_string(output.join("share/content")).unwrap();
    assert!(contents.starts_with("package=/nix/store/"), "{contents}");
    assert!(contents.contains("\nescaped=${literal}\nruntime=$VALUE\n"));
    assert!(fs::read_link(output.join("bin/hello"))
        .unwrap()
        .to_string_lossy()
        .ends_with("/bin/hello"));

    let spec = cix_run::spec::Spec::load(&output).unwrap();
    let manifest = fs::read_to_string(output.join("cix-manifest.json")).unwrap();
    assert!(manifest.starts_with("{\n  \"cixManifest\""), "{manifest}");
    assert!(manifest.ends_with("}\n"), "{manifest}");
    assert_eq!(spec.cix_manifest, 0);
    assert_eq!(spec.select_service(None).unwrap().1.start, ["bin/hello"]);
    assert_eq!(
        spec.select_service(None).unwrap().1.env["PATH"]
            .default
            .as_deref(),
        Some("bin")
    );
}

#[test]
fn overlays_apply_in_order_and_bad_overlay_reports_the_contract() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("first.nix"),
        "final: prev: { cixOverlayFirst = prev.hello; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("second.nix"),
        "final: prev: { cixOverlaySecond = final.cixOverlayFirst; }\n",
    )
    .unwrap();
    let cixfile = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable OVERLAY ./first.nix OVERLAY ./second.nix AS pkgs\nSERVICE fixture\nCOPY ${pkgs.cixOverlaySecond}/bin/hello /bin/hello\nSTART hello\n",
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    assert!(expression.find("/first.nix").unwrap() < expression.find("/second.nix").unwrap());
    let output = build_expression(&expression).unwrap();
    assert!(output.join("bin/hello").exists());

    fs::write(directory.path().join("second.nix"), "{}\n").unwrap();
    let malformed = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let error = build_expression(&malformed).unwrap_err().to_string();
    assert!(error.contains("./second.nix"), "{error}");
    assert!(error.contains("final: prev"), "{error}");
}

#[test]
fn overlay_edits_change_builder_keys_without_repinning_the_base() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable OVERLAY ./overlay.nix AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nRUN printf first > out\nSERVICE app\nCOPY ${build}/out /out\nSTART /bin/true\n",
    )
    .unwrap();
    fs::write(directory.path().join("overlay.nix"), "final: prev: {}\n").unwrap();
    let lock = committed_lock();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    };
    build(&options).unwrap();
    let first: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    fs::write(
        directory.path().join("overlay.nix"),
        "final: prev: { cixOverlayMarker = prev.hello; }\n",
    )
    .unwrap();
    build(&options).unwrap();
    let second: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(
        first.inputs, second.inputs,
        "only --update-lock moves a base pin"
    );
    assert_eq!(
        second.memo.len(),
        2,
        "overlay content changes the chain key"
    );
}

#[test]
fn bare_commands_resolve_against_item_bin_and_explicit_path_replaces_default() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
COPY ${pkgs.coreutils}/bin/true /bin/true
ENV PATH=${pkgs.bash}/bin
START_PRE true
START true
"#,
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let output = build_expression(&expression).unwrap();
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    let service = spec.select_service(None).unwrap().1;
    assert_eq!(service.start, ["bin/true"]);
    assert_eq!(service.start_pre.as_ref().unwrap(), &service.start);
    assert!(service.env["PATH"]
        .default
        .as_deref()
        .is_some_and(|path| path.starts_with("/nix/store/") && path.ends_with("/bin")));
}

#[test]
fn bare_commands_ignore_explicit_path_when_resolving() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
COPY ${pkgs.bash}/bin/bash /bin/bash
ENV PATH=${pkgs.coreutils}/bin
START bash
"#,
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let output = build_expression(&expression).unwrap();
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    let service = spec.select_service(None).unwrap().1;
    assert_eq!(service.start, ["bin/bash"]);
    assert!(service.env["PATH"]
        .default
        .as_deref()
        .is_some_and(|path| path.starts_with("/nix/store/") && path.ends_with("/bin")));
}

#[test]
fn bare_command_failure_lists_the_item_bin_entries() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
COPY ${pkgs.coreutils}/bin/true /bin/true
START definitely-not-in-bin
"#,
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(error.contains("line 4"), "{error}");
    assert!(error.contains("item's bin/"), "{error}");
    assert!(error.contains("true"), "{error}");
}

#[test]
fn run_executes_outside_nix_and_build_interpolation_reaches_the_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("input"), "sandboxed\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/input input
ENV OUTPUT=$PWD/output
RUN <<BUILD
# A RUN heredoc is sent to the same builder shell as a one-line RUN.
cp input "$OUTPUT"
BUILD
SERVICE fixture
COPY ${build}/output /bin/output
START /bin/output
"#,
    )
    .unwrap();
    let mut lock_json = serde_json::to_value(committed_lock()).unwrap();
    lock_json.as_object_mut().unwrap().remove("fetches");
    lock_json.as_object_mut().unwrap().remove("memo");
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock_json).unwrap()),
    )
    .unwrap();

    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let output = &output[0].store_path;
    assert_eq!(
        fs::read_to_string(PathBuf::from(&output).join("bin/output")).unwrap(),
        "sandboxed\n"
    );
    let spec = cix_run::spec::Spec::load(&PathBuf::from(&output)).unwrap();
    assert_eq!(spec.select_service(None).unwrap().1.start[0], "/bin/output");

    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.memo.len(), 1);

    let repeated = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(repeated[0].store_path, *output);
}

#[test]
fn selected_member_executes_only_its_backward_builder_slice() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER wanted
IMPORT ${pkgs.bash}
RUN printf wanted > wanted
BUILDER unrelated
IMPORT ${pkgs.bash}
RUN exit 42
SERVICE api
COPY ${wanted}/wanted /payload
START /bin/true
SERVICE worker
COPY ${unrelated}/missing /missing
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let output = build_family(
        &BuildOptions {
            directory: directory.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &[],
        None,
        Some("api"),
    )
    .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].name, "api");
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("payload")).unwrap(),
        "wanted"
    );
}

#[test]
fn fetch_expect_matches_in_both_forms_and_records_the_declared_hash() {
    let directory = tempfile::tempdir().unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("payload"), "fixed\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    fs::write(
        directory.path().join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FETCH ingredient ${{pkgs.coreutils}}/bin/printf 'fixed\n' > payload EXPECT {expected}
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
FETCH printf 'fixed\n' > payload EXPECT {expected}
SERVICE top
COPY ${{ingredient}}/payload /payload
START /bin/true
SERVICE nested
COPY ${{build}}/payload /payload
START /bin/true
"#,
        ),
    )
    .unwrap();
    let lock = committed_lock();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
    )
    .unwrap();

    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(output.len(), 2);
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.fetches.len(), 2);
    assert!(lock.fetches.values().all(|pin| pin.nar_hash == expected));
}

#[test]
fn warm_fetch_memo_rejects_expect_that_diverges_from_its_recorded_pin() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let first_tree = tempfile::tempdir().unwrap();
    fs::write(first_tree.path().join("release.json"), "release\n").unwrap();
    let second_tree = tempfile::tempdir().unwrap();
    fs::write(second_tree.path().join("release.json"), "release\n").unwrap();
    fs::write(second_tree.path().join("traefik.tar.gz"), "asset\n").unwrap();
    let hash = |path: &Path| {
        cix_common::nix(&["hash", "path", "--mode", "nar", path.to_str().unwrap()])
            .unwrap()
            .trim()
            .to_owned()
    };
    let first = hash(first_tree.path());
    let second = hash(second_tree.path());
    let cixfile = |second_expect: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER release
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
FETCH printf 'release\n' > release.json EXPECT {first}
FETCH printf 'asset\n' > traefik.tar.gz EXPECT {second_expect}
ITEM traefik
COPY ${{release}}/release.json /release.json
COPY ${{release}}/traefik.tar.gz /traefik.tar.gz
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile(&second)).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    };

    build(&options).unwrap();
    let warm_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(warm_lock.step_memo.len(), 2);
    assert_eq!(
        warm_lock
            .fetches
            .iter()
            .find(|(id, _)| id.starts_with("builder:release:1-"))
            .unwrap()
            .1
            .nar_hash,
        second
    );

    fs::write(directory.path().join("Cixfile"), cixfile(&first)).unwrap();
    let error = build(&options).unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("line 5"), "{rendered}");
    assert!(rendered.contains("recorded lock pin"), "{rendered}");
    assert!(
        rendered.contains(&format!("declared {first}")),
        "{rendered}"
    );
    assert!(
        rendered.contains(&format!("lock records {second}")),
        "{rendered}"
    );

    let unchanged: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(
        unchanged
            .fetches
            .iter()
            .find(|(id, _)| id.starts_with("builder:release:1-"))
            .unwrap()
            .1
            .nar_hash,
        second
    );
}

#[test]
fn fetch_expect_mismatch_names_declared_and_actual_hashes() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient ${pkgs.coreutils}/bin/printf payload > payload EXPECT sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\nSERVICE app\nCOPY ${ingredient}/payload /payload\nSTART /bin/true\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("declared sha256-AAAA"), "{rendered}");
    assert!(rendered.contains("fetched sha256-"), "{rendered}");
    assert!(!rendered.contains("--update-lock to accept"), "{rendered}");
}

#[test]
fn imported_cacert_enables_bare_git_over_https() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let cixfile = |cacert: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER fetch
IMPORT ${{pkgs.bash}} ${{pkgs.gitMinimal}}{cacert}
FETCH git ls-remote https://github.com/NixOS/nixpkgs.git HEAD > head
SERVICE result
COPY ${{fetch}}/head /head
START /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("FETCH failed"), "{rendered}");
    assert!(
        rendered.to_ascii_lowercase().contains("certificate"),
        "{rendered}"
    );

    fs::write(directory.path().join("Cixfile"), cixfile(" ${pkgs.cacert}")).unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let head = fs::read_to_string(PathBuf::from(&output[0].store_path).join("head")).unwrap();
    assert!(head.ends_with("\tHEAD\n"), "{head}");
}

#[test]
fn usr_bin_env_shebang_requires_an_imported_env() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let script = directory.path().join("script");
    fs::write(&script, "#!/usr/bin/env bash\nprintf shebang-ok\n").unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).unwrap();
    let cixfile = |coreutils: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${{pkgs.bash}}{coreutils}
COPY ${{src}}/script script
RUN ./script > output
SERVICE result
COPY ${{build}}/output /output
START /bin/true
"#
        )
    };

    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("RUN failed"), "{rendered}");
    assert!(rendered.contains("/usr/bin/env"), "{rendered}");
    assert!(rendered.contains("IMPORT ${pkgs.coreutils}"), "{rendered}");

    fs::write(
        directory.path().join("Cixfile"),
        cixfile(" ${pkgs.coreutils}"),
    )
    .unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&output[0].store_path).join("output")).unwrap(),
        "shebang-ok"
    );
}

#[test]
fn newly_consumed_path_reruns_the_chain_and_extends_its_record() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let cixfile = |extra_copy: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
RUN printf x >> runs; printf one > one; printf two > two
SERVICE result
COPY ${{build}}/runs /runs
COPY ${{build}}/one /one
{extra_copy}START /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("runs")).unwrap(),
        "x"
    );

    fs::write(
        directory.path().join("Cixfile"),
        cixfile("COPY ${build}/two /two\n"),
    )
    .unwrap();
    let second = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&second[0].store_path).join("runs")).unwrap(),
        "xx"
    );
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.memo.len(), 1);
    assert_eq!(
        lock.memo
            .values()
            .next()
            .unwrap()
            .paths
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one", "runs", "two"]
    );
}

#[test]
fn edited_plan_reconciles_repeated_copy_in_warm_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("input"), "payload\n").unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let cixfile = |builder_suffix: &str, item_suffix: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{src}}/input input
RUN cp input first
{builder_suffix}ITEM result
COPY ${{build}}/first /first
{item_suffix}"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("", "")).unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    };
    build(&options).unwrap();

    fs::write(
        directory.path().join("Cixfile"),
        cixfile(
            "COPY ${src}/input input\nRUN cp input second\n",
            "COPY ${build}/second /second\n",
        ),
    )
    .unwrap();
    let edited = build(&options).unwrap();
    let item = PathBuf::from(&edited[0].store_path);
    assert_eq!(fs::read_to_string(item.join("first")).unwrap(), "payload\n");
    assert_eq!(
        fs::read_to_string(item.join("second")).unwrap(),
        "payload\n"
    );
}

#[test]
fn automatic_fetch_pins_only_consumed_paths_and_cold_replays_its_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
FETCH test ! -e fetch-ran; touch fetch-ran; printf payload > wanted; printf incidental > ignored
RUN printf suffix >> wanted; cp wanted result
SERVICE result
COPY ${build}/result /result
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let pin = lock.fetches.values().next().unwrap();
    assert_eq!(
        pin.paths.keys().map(String::as_str).collect::<Vec<_>>(),
        ["result"]
    );
    // The replay snapshot already contains fetch-ran. Re-executing FETCH would fail
    // its first command, so a successful cold build proves no fetch process spawned.
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("result")).unwrap(),
        "payloadsuffix"
    );
}

#[test]
fn cold_replays_a_top_level_fetch_snapshot_without_executing_fetch() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FETCH ingredient test ! -e fetch-ran; printf ran > fetch-ran; printf payload > payload
SERVICE result
COPY ${ingredient}/payload /payload
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("payload")).unwrap(),
        "payload"
    );
}

#[test]
fn formatting_keeps_fetch_identity_snapshot_lookup_and_item_output() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = "FROM\tgithub:NixOS/nixpkgs/nixos-unstable\tAS\tpkgs\nFETCH ingredient test ! -e fetch-ran; : > fetch-ran; printf payload > payload\nSERVICE\tresult\nCOPY\t${ingredient}/payload\t/payload\nSTART\t/bin/true\n";
    fs::write(directory.path().join("Cixfile"), cixfile).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    };

    let first = build(&options).unwrap();
    let first_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let formatted = cix_cixfile::fmt::format(cixfile).unwrap();
    assert_ne!(cixfile, formatted);
    fs::write(directory.path().join("Cixfile"), formatted).unwrap();

    let formatted = build(&BuildOptions {
        cold: true,
        ..options
    })
    .unwrap();
    let formatted_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let first_pin = first_lock.fetches.get("ingredient").unwrap();
    let formatted_pin = formatted_lock.fetches.get("ingredient").unwrap();
    assert_eq!(first_pin.key(), formatted_pin.key());
    assert_eq!(first_pin.snapshot_nar_hash, formatted_pin.snapshot_nar_hash);
    assert_eq!(
        first_lock.outputs["result"].source_hash,
        formatted_lock.outputs["result"].source_hash
    );
    assert_eq!(
        fs::read_to_string(Path::new(&first[0].store_path).join("payload")).unwrap(),
        fs::read_to_string(Path::new(&formatted[0].store_path).join("payload")).unwrap()
    );
    assert_eq!(first[0].store_path, formatted[0].store_path);
}

#[test]
fn newly_consumed_fetch_path_extends_an_automatic_pin() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = |extra_copy: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
FETCH printf one > one; printf two > two
RUN cp one result
SERVICE result
COPY ${{build}}/result /result
{extra_copy}START /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    };
    build(&options).unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        cixfile("COPY ${build}/two /two\n"),
    )
    .unwrap();
    build(&options).unwrap();

    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(
        lock.fetches
            .values()
            .next()
            .unwrap()
            .paths
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["result", "two"]
    );
}

#[test]
fn update_lock_keeps_consumed_result_stable_across_unconsumed_timestamped_fetch_output() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils} ${pkgs.findutils}
FETCH mkdir -p .npm/_logs; date +%s%N > .npm/_logs/$(date +%s%N)-debug.log; find .npm/_logs -type f -print >/dev/null; printf payload > result
SERVICE result
COPY ${build}/result /result
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let first_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    let second_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let first_pin = first_lock.fetches.values().next().unwrap();
    let second_pin = second_lock.fetches.values().next().unwrap();
    assert_ne!(
        first_pin.snapshot_nar_hash, second_pin.snapshot_nar_hash,
        "CIP-94 records the complete immediate post-FETCH workspace"
    );
    assert_eq!(first_pin.paths, second_pin.paths);
    assert_eq!(first_lock.memo, second_lock.memo);
    assert_eq!(first_lock.step_memo, second_lock.step_memo);
    assert_eq!(
        first_lock.outputs["result"].store_path,
        second_lock.outputs["result"].store_path
    );
    assert_eq!(
        second_pin
            .paths
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["result"]
    );
    assert!(first_pin.volatile.is_empty());
    assert!(second_pin.volatile.is_empty());
}

#[test]
fn warm_source_edit_after_fetch_reuses_the_pinned_prefix() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("manifest"), "manifest\n").unwrap();
    fs::write(directory.path().join("source"), "v1\n").unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("manifest"), "manifest\n").unwrap();
    fs::write(expected_tree.path().join("payload"), "fixed\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    fs::write(
        directory.path().join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{src}}/manifest manifest
FETCH printf 'fixed\n' > payload EXPECT {expected}
COPY ${{src}}/source source
RUN cp source output
SERVICE result
COPY ${{build}}/output /output
START /bin/true
"#
        ),
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("output")).unwrap(),
        "v1\n"
    );

    fs::write(directory.path().join("source"), "v2\n").unwrap();
    let second = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&second[0].store_path).join("output")).unwrap(),
        "v2\n"
    );
}

#[test]
fn fetch_self_observation_reverts_partial_state_and_preserves_cold_and_pin_checks() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("value"), "one\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/value value
FETCH test ! -e foo && test ! -e bar && cat value > foo && printf side > bar
RUN cat foo > result
SERVICE result
COPY ${build}/result /result
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    };

    build(&options).unwrap();
    let builder_workspace = fs::read_dir(workspace.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("work");
    fs::remove_file(builder_workspace.join("bar")).unwrap();
    fs::write(directory.path().join("value"), "two\n").unwrap();

    let mismatch = build(&options).unwrap_err().to_string();
    assert!(
        mismatch.contains("FETCH consumed-path mismatch") && mismatch.contains("--update-lock"),
        "{mismatch}"
    );

    let updated = build(&BuildOptions {
        update_lock: Some("build".into()),
        ..options.clone()
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&updated[0].store_path).join("result")).unwrap(),
        "two\n"
    );
    let warm_lock = fs::read(directory.path().join("Cixfile.lock")).unwrap();
    let cold = build(&BuildOptions {
        cold: true,
        ..options
    })
    .unwrap();
    assert_eq!(
        updated, cold,
        "warm and cold must materialize identical items"
    );
    assert_eq!(
        warm_lock,
        fs::read(directory.path().join("Cixfile.lock")).unwrap(),
        "warm and cold must record byte-identical traces"
    );
}

#[test]
fn changed_step_before_fetch_reuses_its_builder_underlay() {
    let directory = tempfile::tempdir().unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("required"), "present\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    let cixfile = |middle: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
RUN printf 'present\n' > required
RUN {middle}
FETCH test -f required EXPECT {expected}
SERVICE result
COPY ${{build}}/required /required
START /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("true")).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();

    fs::write(directory.path().join("Cixfile"), cixfile("test 1 = 1")).unwrap();
    let rebuilt = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&rebuilt[0].store_path).join("required")).unwrap(),
        "present\n"
    );
}

#[test]
fn warm_rerun_starts_on_its_builder_end_state_while_cold_does_not() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("source"), "v1\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
RUN printf 'before\n' >> history
COPY ${src}/source source
RUN cat source >> history && cp history output
SERVICE result
COPY ${build}/output /output
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("output")).unwrap(),
        "before\nv1\n"
    );

    fs::write(directory.path().join("source"), "v2\n").unwrap();
    let warm = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&warm[0].store_path).join("output")).unwrap(),
        "before\nv1\nv2\n"
    );

    let cold = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
        allow_secret: false,
        workspace_directory: workspace.path().to_owned(),
    })
    .unwrap_err()
    .to_string();
    assert!(
        cold.contains("line 7: recorded read set differs between warm and cold at \"history\"")
            && cold.contains("RUN cat source >> history"),
        "{cold}"
    );
}

#[test]
fn bare_and_explicit_local_copy_contexts_are_byte_identical() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("payload"), "same context\n").unwrap();
    let bare = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nCOPY payload /share/payload\nSTART /bin/true\n",
    )
    .unwrap();
    let explicit = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nSERVICE fixture\nCOPY ${src}/payload /share/payload\nSTART /bin/true\n",
    )
    .unwrap();
    let bare = build_expression(
        &generate_nix(&bare, directory.path(), &committed_lock(), "x86_64-linux").unwrap(),
    )
    .unwrap();
    let explicit = build_expression(
        &generate_nix(
            &explicit,
            directory.path(),
            &committed_lock(),
            "x86_64-linux",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(nar_hash(&bare), nar_hash(&explicit));
    assert_eq!(
        fs::read_to_string(bare.join("share/payload")).unwrap(),
        "same context\n"
    );
    let manifest = cix_run::spec::Spec::load(&bare).unwrap();
    assert_eq!(manifest.kind, cix_run::spec::ManifestKind::Service);
}

#[test]
fn cix_item_from_copies_a_lock_pinned_tag_and_rejects_a_bad_nar_hash() {
    let state = tempfile::tempdir().unwrap();
    let registry = TestRegistry(cix_index::Store::open(state.path().to_owned()).unwrap());

    let missing = tempfile::tempdir().unwrap();
    fs::write(
        missing.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM family/missing:v1 AS missing
SERVICE consumer
COPY ${missing}/payload /payload
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        missing.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let missing_error = build_with_registry(
        &BuildOptions {
            directory: missing.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &registry,
    )
    .unwrap_err()
    .to_string();
    assert!(
        missing_error.contains("pull it or tag it first"),
        "{missing_error}"
    );

    let producer = tempfile::tempdir().unwrap();
    fs::write(
        producer.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
ITEM source
FILE /payload <<EOF
first
EOF
"#,
    )
    .unwrap();
    fs::write(
        producer.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let producer_output = build(&BuildOptions {
        directory: producer.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: test_workspace_directory(),
    })
    .unwrap()
    .remove(0)
    .store_path;
    assert!(!Path::new(&producer_output)
        .join("cix-manifest.json")
        .exists());
    let manifest_error = cix_run::spec::Spec::load(Path::new(&producer_output))
        .unwrap_err()
        .to_string();
    assert!(
        manifest_error.contains("manifest-less ITEM (D68)"),
        "{manifest_error}"
    );
    assert!(manifest_error.contains("SERVICE/APP"), "{manifest_error}");
    cix_index::tag(&registry.0, &producer_output, "family/source:v1", None).unwrap();

    let consumer = tempfile::tempdir().unwrap();
    fs::write(
        consumer.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM family/source:v1 AS source
SERVICE consumer
COPY ${source}/payload /payload
START /bin/true
"#,
    )
    .unwrap();
    fs::write(
        consumer.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let first = build_with_registry(
        &BuildOptions {
            directory: consumer.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &registry,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&first[0].store_path).join("payload")).unwrap(),
        "first\n"
    );
    let mut lock: LockFile =
        serde_json::from_slice(&fs::read(consumer.path().join("Cixfile.lock")).unwrap()).unwrap();
    let pin = lock.artifacts["family/source:v1"].clone();
    assert_eq!(pin.store_path, producer_output);

    let moved_tree = tempfile::tempdir().unwrap();
    fs::write(moved_tree.path().join("payload"), "moved\n").unwrap();
    let moved = cix_common::nix(&["store", "add-path", moved_tree.path().to_str().unwrap()])
        .unwrap()
        .trim()
        .to_owned();
    cix_index::tag(&registry.0, &moved, "family/source:v1", None).unwrap();

    let pinned = build_with_registry(
        &BuildOptions {
            directory: consumer.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &registry,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&pinned[0].store_path).join("payload")).unwrap(),
        "first\n"
    );

    let updated = build_with_registry(
        &BuildOptions {
            directory: consumer.path().to_owned(),
            update_lock: Some("source".into()),
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &registry,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&updated[0].store_path).join("payload")).unwrap(),
        "moved\n"
    );

    lock.artifacts.insert(
        "family/source:v1".into(),
        ArtifactPin {
            store_path: moved,
            nar_hash: "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
        },
    );
    fs::write(
        consumer.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
    )
    .unwrap();
    let error = build_with_registry(
        &BuildOptions {
            directory: consumer.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: test_workspace_directory(),
        },
        &registry,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("narHash mismatch"), "{error}");
}

fn nar_hash(path: &std::path::Path) -> String {
    let output = Command::new("nix-store")
        .args(["--query", "--hash"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}
