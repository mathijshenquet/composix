//! Sample the D47(e) clean-rebuild bridge check over every shipped Cixfile.
//!
//! This is intentionally ignored: examples may need Nix downloads and `FETCH` network access.
//! Run the prescribed gate with:
//! `devenv shell -- cargo test -p cix --test cold_audit -- --ignored`
//!
//! Set `COLD_AUDIT=<corpus-pair>` to fetch and audit one `corpus/migrate/docker/<corpus-pair>`
//! Cixfile as well. For example: `COLD_AUDIT=adminer devenv shell -- cargo test -p cix
//! --test cold_audit -- --ignored`.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type MemberMap = BTreeMap<String, String>;

struct Audit {
    root: PathBuf,
    state: PathBuf,
    workspaces: PathBuf,
}

impl Audit {
    fn new(root: PathBuf, temp: &Path) -> Self {
        let state = temp.join("state");
        let workspaces = temp.join("workspaces");
        fs::create_dir_all(&state).expect("creating isolated cix state");
        fs::create_dir_all(&workspaces).expect("creating isolated builder workspaces");
        Self {
            root,
            state,
            workspaces,
        }
    }

    fn command(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cix"))
            .args(args)
            .current_dir(&self.root)
            .env("CIX_STATE_DIR", &self.state)
            .env("CIX_BUILD_WORKSPACE_DIR", &self.workspaces)
            .output()
            .unwrap_or_else(|error| panic!("running `cix {}`: {error}", args.join(" ")))
    }

    fn build(&self, directory: &Path, cold: bool) -> Result<MemberMap, String> {
        let mut args = vec!["build".to_owned()];
        if cold {
            args.push("--cold".to_owned());
        }
        args.push(directory.display().to_string());
        let output = self.command(&args);
        if !output.status.success() {
            return Err(command_failure(&args, &output));
        }
        serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "`cix {}` did not emit a JSON member map: {error}\nstdout:\n{}\nstderr:\n{}",
                args.join(" "),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )
        })
    }

    fn tag_nginx(&self, directory: &Path) {
        let args = vec![
            "build".to_owned(),
            "-t".to_owned(),
            "v1".to_owned(),
            directory.display().to_string(),
        ];
        let output = self.command(&args);
        assert!(
            output.status.success(),
            "{}",
            command_failure(&args, &output)
        );
    }
}

fn command_failure(args: &[String], output: &Output) -> String {
    format!(
        "`cix {}` failed with {}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

fn audit_pair(audit: &Audit, label: &str, directory: &Path) -> Result<(), String> {
    let warm = audit.build(directory, false)?;
    let cold = audit.build(directory, true)?;
    let members = warm
        .keys()
        .chain(cold.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for member in members {
        let warm_path = warm.get(&member).map(String::as_str).unwrap_or("<missing>");
        let cold_path = cold.get(&member).map(String::as_str).unwrap_or("<missing>");
        if warm_path != cold_path {
            return Err(format!(
                "cold audit mismatch for {label}, member {member:?}: warm {warm_path:?}; cold {cold_path:?}"
            ));
        }
    }
    Ok(())
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("resolving repository root")
}

fn test_tempdir(name: &str) -> tempfile::TempDir {
    let base = repository_root().join("target/test-tmp");
    fs::create_dir_all(&base).expect("creating target/test-tmp");
    tempfile::Builder::new()
        .prefix(&format!("cix-cold-audit-{name}-"))
        .tempdir_in(base)
        .expect("creating audit tempdir")
}

fn cixfile_directories(root: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, directories: &mut Vec<PathBuf>) {
        if directory.join("Cixfile").is_file() {
            directories.push(directory.to_owned());
        }
        let mut entries = fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
            .map(|entry| entry.expect("reading directory entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if entry
                .file_type()
                .expect("reading directory entry type")
                .is_dir()
            {
                visit(&path, directories);
            }
        }
    }

    let mut directories = Vec::new();
    visit(root, &mut directories);
    directories
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination)
        .unwrap_or_else(|error| panic!("creating {}: {error}", destination.display()));
    let mut entries = fs::read_dir(source)
        .unwrap_or_else(|error| panic!("reading {}: {error}", source.display()))
        .map(|entry| entry.expect("reading directory entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("reading source entry type");
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else if file_type.is_symlink() {
            symlink(
                fs::read_link(&source_path)
                    .unwrap_or_else(|error| panic!("reading {}: {error}", source_path.display())),
                &destination_path,
            )
            .unwrap_or_else(|error| panic!("linking {}: {error}", destination_path.display()));
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "copying {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
            fs::set_permissions(
                &destination_path,
                fs::metadata(&source_path)
                    .unwrap_or_else(|error| panic!("reading {}: {error}", source_path.display()))
                    .permissions(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "setting {} permissions: {error}",
                    destination_path.display()
                )
            });
        }
    }
}

fn audit_examples() {
    let root = repository_root();
    let source_examples = root.join("examples");
    let example_directories = cixfile_directories(&source_examples);
    assert!(
        !example_directories.is_empty(),
        "no examples/**/Cixfile files found"
    );

    let temp = test_tempdir("examples");
    let copied_examples = temp.path().join("examples");
    copy_tree(&source_examples, &copied_examples);
    let audit = Audit::new(root, temp.path());

    // `examples/build/from-item` is a complete consumer, whose documented fixture first
    // tags the producer. The tag lives only in this audit's isolated state directory.
    audit.tag_nginx(&copied_examples.join("pack/nginx"));

    for source_directory in example_directories {
        let relative = source_directory
            .strip_prefix(&source_examples)
            .expect("example directory is below examples");
        // These cargo FETCHes are pin-instability exhibits (dozzle class). Two
        // independent clean `--update-lock build` runs disagreed, so neither can
        // honestly be re-pinned or compared by this deterministic sweep.
        if matches!(
            relative.to_str(),
            Some("build/projB") | Some("build/projB-chef")
        ) {
            eprintln!(
                "cold audit excludes examples/{}: cargo FETCH pin-instability exhibit (dozzle class)",
                relative.display()
            );
            continue;
        }
        let directory = copied_examples.join(relative);
        let label = format!("examples/{}", relative.display());
        audit_pair(&audit, &label, &directory).unwrap_or_else(|error| panic!("{error}"));
    }
}

#[test]
#[ignore = "networked D47(e) sample; run the cold-audit gate explicitly"]
fn every_example_matches_a_clean_rebuild() {
    audit_examples();
}

#[test]
#[ignore = "manual corpus sample selected through COLD_AUDIT"]
fn selected_corpus_pair_matches_a_clean_rebuild() {
    let Ok(pair) = env::var("COLD_AUDIT") else {
        eprintln!("skipping corpus sample: set COLD_AUDIT=<corpus-pair>");
        return;
    };
    assert!(
        !pair.is_empty() && !pair.contains('/') && !pair.contains(std::path::MAIN_SEPARATOR),
        "COLD_AUDIT must name one Docker corpus case, got {pair:?}"
    );

    let root = repository_root();
    let corpus_root = root.join("corpus/migrate");
    let corpus = corpus_root.join("docker");
    let fetch = Command::new("bash")
        .args(["fetch.sh", &pair])
        .current_dir(&corpus_root)
        .output()
        .expect("running corpus/migrate/docker/fetch.sh");
    assert!(
        fetch.status.success(),
        "`corpus/migrate/docker/fetch.sh {pair}` failed with {}\nstdout:\n{}\nstderr:\n{}",
        fetch.status,
        String::from_utf8_lossy(&fetch.stdout),
        String::from_utf8_lossy(&fetch.stderr),
    );

    let source_directory = corpus.join(&pair);
    assert!(
        source_directory.join("Cixfile").is_file(),
        "COLD_AUDIT={pair:?} has no corpus/migrate/docker/{pair}/Cixfile"
    );
    let temp = test_tempdir(&format!("corpus-{pair}"));
    let directory = temp.path().join(&pair);
    copy_tree(&source_directory, &directory);
    let audit = Audit::new(root, temp.path());
    audit_pair(&audit, &format!("corpus/migrate/docker/{pair}"), &directory)
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
#[ignore = "proof that the audit reports a clean-rebuild mismatch"]
fn nondeterministic_builder_is_rejected() {
    let temp = test_tempdir("negative");
    fs::write(
        temp.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
RUN ${pkgs.coreutils}/bin/date +%s%N > x
SERVICE fixture
COPY ${build}/x /share/x
START /bin/true
"#,
    )
    .expect("writing nondeterministic Cixfile");
    fs::write(
        temp.path().join("Cixfile.lock"),
        include_str!("../../../examples/pack/nginx/Cixfile.lock"),
    )
    .expect("writing nondeterministic Cixfile lock");

    let audit = Audit::new(repository_root(), temp.path());
    let error = audit_pair(&audit, "temporary nondeterministic fixture", temp.path())
        .expect_err("the cold audit must reject a timestamped builder output");
    assert!(
        error.contains("COPY ${build}/x (line 6) differs between warm and cold"),
        "the cold audit did not retain its useful mismatch diagnostic:\n{error}"
    );
}
