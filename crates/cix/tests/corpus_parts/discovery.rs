use super::super::*;
pub(crate) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn corpus_dir() -> PathBuf {
    repository_root().join("corpus/migrate")
}

pub(crate) fn browser_dir() -> PathBuf {
    repository_root().join("docs/corpus")
}

pub(crate) fn corpus_files(directory: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
        let mut entries = fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
            .map(|entry| entry.expect("reading corpus artifact entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            if entry.is_dir() {
                visit(&entry, files);
            } else if entry.is_file() {
                files.push(entry);
            }
        }
    }

    let mut files = Vec::new();
    visit(directory, &mut files);
    files.retain(|path| !path.starts_with(directory.join("context")));
    files
}

pub(crate) fn relative_name(directory: &Path, path: &Path) -> String {
    path.strip_prefix(directory)
        .expect("corpus artifact remains under its case directory")
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn read_artifact(directory: &Path, path: &Path, collapsed: bool) -> Artifact {
    Artifact {
        name: relative_name(directory, path),
        content: fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
        collapsed,
    }
}

pub(crate) fn is_upstream_artifact(name: &str) -> bool {
    name == "Dockerfile"
        || name.starts_with("upstream-")
        || name == "SOURCE"
        || name == "context.files"
}

pub(crate) fn is_collapsed_cix_artifact(name: &str) -> bool {
    name == "check.sh"
        || name == "receipt.md"
        || name.ends_with("Cixfile.lock")
        || name.ends_with("Cixfile.dissolved.lock")
}

pub(crate) fn read_upstream_artifacts(directory: &Path) -> Vec<Artifact> {
    corpus_files(directory)
        .into_iter()
        .filter(|path| is_upstream_artifact(&relative_name(directory, path)))
        .filter(|path| relative_name(directory, path) != "context.files")
        .map(|path| read_artifact(directory, &path, false))
        .collect()
}

pub(crate) fn read_cix_artifacts(directory: &Path) -> Vec<Artifact> {
    let mut artifacts = corpus_files(directory)
        .into_iter()
        .filter_map(|path| {
            let name = relative_name(directory, &path);
            (!is_upstream_artifact(&name) && name != "GAPS.md")
                .then(|| read_artifact(directory, &path, is_collapsed_cix_artifact(&name)))
        })
        .collect::<Vec<_>>();
    artifacts.sort_by_key(|artifact| {
        (
            artifact.name.rsplit('/').next() != Some("Cixfile"),
            artifact.name.clone(),
        )
    });
    artifacts
}

pub(crate) fn read_context_files(directory: &Path) -> Option<Vec<ContextFile>> {
    let path = directory.join("context.files");
    path.is_file().then(|| {
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
            .lines()
            .map(|line| {
                let (relative_path, bytes) = line.split_once('\t').unwrap_or_else(|| {
                    panic!(
                        "invalid context manifest line in {}: {line:?}",
                        path.display()
                    )
                });
                ContextFile {
                    path: relative_path.to_owned(),
                    bytes: bytes.parse().unwrap_or_else(|error| {
                        panic!("invalid context size in {}: {error}", path.display())
                    }),
                }
            })
            .collect()
    })
}

pub(crate) fn read_gap_panel(directory: &Path) -> Option<GapPanel> {
    let path = directory.join("GAPS.md");
    path.is_file().then(|| {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        let mut generated = None;
        let mut status = None;
        let body = source
            .lines()
            .filter(|line| {
                if let Some(value) = line.strip_prefix("Generated:") {
                    generated = Some(value.trim().to_owned());
                    false
                } else if let Some(value) = line.strip_prefix("Status:") {
                    status = Some(value.trim().to_owned());
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        GapPanel {
            generated: generated.unwrap_or_else(|| "not recorded".to_owned()),
            status: status.unwrap_or_else(|| "not recorded".to_owned()),
            body,
        }
    })
}

pub(crate) fn ledger_grades() -> BTreeMap<String, LedgerGrade> {
    let path = repository_root().join("docs/corpus.md");
    let ledger = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let mut grades = BTreeMap::new();
    for line in ledger.lines().filter(|line| line.starts_with('|')) {
        let cells = line
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        if cells.len() < 6 {
            continue;
        }
        let Some(after_prefix) = cells[5].split("../corpus/migrate/").nth(1) else {
            continue;
        };
        let Some(case) = after_prefix.split("/receipt.md").next() else {
            continue;
        };
        grades
            .entry(case.to_owned())
            .or_insert_with(|| LedgerGrade {
                receipt_status: plain_markdown(cells[2]),
                ribbon: plain_markdown(cells[3]),
                evidence: plain_markdown(cells[5]),
            });
    }
    grades
}

pub(crate) fn plain_markdown(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some(open) = rest.find('[') {
        output.push_str(&rest[..open]);
        let link = &rest[open + 1..];
        let Some(middle) = link.find("](") else {
            output.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let Some(close) = link[middle + 2..].find(')') else {
            output.push_str(&rest[open..]);
            rest = "";
            break;
        };
        output.push_str(&link[..middle]);
        rest = &link[middle + 2 + close + 1..];
    }
    output.push_str(rest);
    output.replace('`', "")
}

pub(crate) fn load_cases() -> Vec<Case> {
    let grades = ledger_grades();
    ["docker", "k8s"]
        .into_iter()
        .flat_map(|axis| {
            let mut directories = fs::read_dir(corpus_dir().join(axis))
                .unwrap_or_else(|error| panic!("reading corpus/migrate/{axis}: {error}"))
                .map(|entry| entry.expect("reading corpus case entry").path())
                .filter(|path| path.is_dir())
                .collect::<Vec<_>>();
            directories.sort();
            directories
                .into_iter()
                .map(move |directory| (axis, directory))
        })
        .filter_map(|(axis, directory)| {
            let cix = read_cix_artifacts(&directory);
            if cix.is_empty() {
                return None;
            }
            let name = directory
                .file_name()
                .expect("corpus case has a name")
                .to_string_lossy()
                .into_owned();
            let upstream = read_upstream_artifacts(&directory);
            assert!(
                !upstream.is_empty(),
                "corpus/migrate/{axis}/{name} has cix artifacts but no checked-in upstream artifact"
            );
            assert!(
                directory.join("receipt.md").is_file(),
                "corpus/migrate/{axis}/{name} has no receipt.md"
            );
            let key = format!("{axis}/{name}");
            let grade = grades.get(&key).unwrap_or_else(|| {
                panic!("corpus/migrate/{key} has no ribbon/evidence row in docs/corpus.md")
            });
            Some(Case {
                axis: axis.to_owned(),
                grade: grade.clone(),
                name,
                upstream,
                cix,
                context_files: read_context_files(&directory),
                gap: read_gap_panel(&directory),
            })
        })
        .collect()
}
