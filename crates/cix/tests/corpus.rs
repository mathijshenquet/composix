//! Deterministic side-by-side browser generated from `corpus/migrate/`.
//!
//! Run `cargo test --test corpus -- --ignored generate_corpus_browser` to update it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const REPOSITORY_URL: &str = "https://github.com/mathijshenquet/composix/blob/main";
const PAGE_STYLE: &str = r#"
    :root { color-scheme: light dark; font-family: system-ui, sans-serif; }
    body { max-width: 96rem; margin: 0 auto; padding: 1.5rem; line-height: 1.5; }
    a { color: #2878c7; }
    .crumbs, .status, .source { color: #667085; }
    .artifacts { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 1rem; align-items: start; }
    .column { min-width: 0; border: 1px solid #98a2b3; border-radius: .5rem; padding: 0 1rem 1rem; }
    .file { margin-top: 1rem; }
    .file h3 { margin-bottom: .35rem; font-size: 1rem; }
    pre { box-sizing: border-box; max-width: 100%; margin: 0; padding: 1rem; overflow-x: auto; border-radius: .35rem; background: #111827; color: #f9fafb; white-space: pre; tab-size: 2; }
    code { font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }
    table { width: 100%; border-collapse: collapse; }
    th, td { padding: .6rem; border-bottom: 1px solid #98a2b3; text-align: left; vertical-align: top; }
    th { white-space: nowrap; }
    footer { margin-top: 2rem; color: #667085; }
"#;

#[derive(Debug, PartialEq, Eq)]
struct GeneratedFile {
    name: String,
    content: String,
}

#[derive(Clone)]
struct Artifact {
    name: String,
    content: String,
}

#[derive(Clone)]
struct LedgerGrade {
    receipt_status: String,
    ribbon: String,
    evidence: String,
}

struct Case {
    name: String,
    upstream: Vec<Artifact>,
    cix: Vec<Artifact>,
    grade: LedgerGrade,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_dir() -> PathBuf {
    repository_root().join("corpus/migrate")
}

fn browser_dir() -> PathBuf {
    repository_root().join("docs/corpus")
}

fn read_artifacts(directory: &Path, names: &[&str]) -> Vec<Artifact> {
    names
        .iter()
        .filter_map(|name| {
            let path = directory.join(name);
            path.is_file().then(|| Artifact {
                name: (*name).to_owned(),
                content: fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
            })
        })
        .collect()
}

fn ledger_grades() -> BTreeMap<String, LedgerGrade> {
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

fn plain_markdown(input: &str) -> String {
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

fn load_cases() -> Vec<Case> {
    let grades = ledger_grades();
    let mut directories = fs::read_dir(corpus_dir())
        .expect("reading corpus/migrate")
        .map(|entry| entry.expect("reading corpus case entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();

    directories
        .into_iter()
        .filter_map(|directory| {
            let cix = read_artifacts(&directory, &["Cixfile", "default.nix", "compose.json"]);
            if cix.is_empty() {
                return None;
            }
            let name = directory
                .file_name()
                .expect("corpus case has a name")
                .to_string_lossy()
                .into_owned();
            let upstream = read_artifacts(
                &directory,
                &[
                    "Dockerfile",
                    "upstream-compose.yaml",
                    "upstream-compose.yml",
                    "upstream-cronjob.yaml",
                ],
            );
            assert!(
                !upstream.is_empty(),
                "corpus/migrate/{name} has cix artifacts but no checked-in upstream artifact"
            );
            assert!(
                directory.join("receipt.md").is_file(),
                "corpus/migrate/{name} has no receipt.md"
            );
            let grade = grades.get(&name).unwrap_or_else(|| {
                panic!("corpus/migrate/{name} has no ribbon/evidence row in docs/corpus.md")
            });
            Some(Case {
                grade: grade.clone(),
                name,
                upstream,
                cix,
            })
        })
        .collect()
}

fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn page_start(title: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>{PAGE_STYLE}  </style>\n</head>\n<body>\n",
        html_escape(title)
    )
}

fn render_artifacts(artifacts: &[Artifact]) -> String {
    let mut output = String::new();
    for artifact in artifacts {
        write!(
            output,
            "      <section class=\"file\">\n        <h3>{}</h3>\n        <pre><code>{}</code></pre>\n      </section>\n",
            html_escape(&artifact.name),
            html_escape(&artifact.content)
        )
        .expect("rendering artifact");
    }
    output
}

fn ribbon_and_evidence(case: &Case) -> (&str, &str) {
    (case.grade.ribbon.as_str(), case.grade.evidence.as_str())
}

fn render_case(case: &Case) -> String {
    let title = format!("{} — migration corpus", case.name);
    let mut page = page_start(&title);
    let (ribbon, evidence) = ribbon_and_evidence(case);
    let receipt_url = format!("{REPOSITORY_URL}/corpus/migrate/{}/receipt.md", case.name);
    let source_url = format!("{REPOSITORY_URL}/corpus/migrate/{}/SOURCE", case.name);
    write!(
        page,
        "  <nav class=\"crumbs\"><a href=\"index.html\">Corpus browser</a> / {name}</nav>\n  <h1>{name}</h1>\n  <p class=\"status\"><strong>docs/corpus.md ribbon:</strong> {ribbon} · <strong>Evidence:</strong> {evidence}</p>\n  <p class=\"status\"><strong>Receipt status:</strong> {receipt_status} · <a href=\"{receipt_url}\">Full migration receipt</a></p>\n  <p class=\"source\"><a href=\"{source_url}\">Pinned upstream source notes</a></p>\n  <main class=\"artifacts\">\n    <section class=\"column\">\n      <h2>Upstream</h2>\n{upstream}    </section>\n    <section class=\"column\">\n      <h2>composix</h2>\n{cix}    </section>\n  </main>\n  <footer>Generated from <code>corpus/migrate/{name}</code>. Do not edit this page by hand.</footer>\n</body>\n</html>\n",
        name = html_escape(&case.name),
        ribbon = html_escape(ribbon),
        evidence = html_escape(evidence),
        receipt_status = html_escape(&case.grade.receipt_status),
        receipt_url = html_escape(&receipt_url),
        source_url = html_escape(&source_url),
        upstream = render_artifacts(&case.upstream),
        cix = render_artifacts(&case.cix),
    )
    .expect("rendering case page");
    page
}

fn render_index(cases: &[Case]) -> String {
    let mut page = page_start("Migration corpus browser — composix");
    page.push_str(
        "  <nav class=\"crumbs\"><a href=\"../index.html\">composix docs</a> / corpus</nav>\n  <h1>Migration corpus</h1>\n  <p>One living corpus, generated directly from the checked-in upstream and composix artifacts. Each case opens as a two-column comparison; its receipt records what was actually verified and what remains open.</p>\n  <p class=\"status\"><strong>Do not edit these pages.</strong> Regenerate with <code>cargo test --test corpus -- --ignored generate_corpus_browser</code>. Normal tests reject drift.</p>\n  <table>\n    <thead><tr><th>Case</th><th>docs/corpus.md ribbon</th><th>Evidence class</th></tr></thead>\n    <tbody>\n",
    );
    for case in cases {
        let (ribbon, evidence) = ribbon_and_evidence(case);
        writeln!(
            page,
            "      <tr><td><a href=\"{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>",
            html_escape(&case.name),
            html_escape(&case.name),
            html_escape(ribbon),
            html_escape(evidence)
        )
        .expect("rendering corpus index row");
    }
    page.push_str(
        "    </tbody>\n  </table>\n  <footer>Generated from <code>corpus/migrate/</code> and the ribbons in <code>docs/corpus.md</code>.</footer>\n</body>\n</html>\n",
    );
    page
}

fn render_browser() -> Vec<GeneratedFile> {
    let cases = load_cases();
    let mut files = Vec::with_capacity(cases.len() + 1);
    files.push(GeneratedFile {
        name: "index.html".to_owned(),
        content: render_index(&cases),
    });
    files.extend(cases.iter().map(|case| GeneratedFile {
        name: format!("{}.html", case.name),
        content: render_case(case),
    }));
    files
}

#[test]
#[ignore = "run explicitly to update docs/corpus/"]
fn generate_corpus_browser() {
    let directory = browser_dir();
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("removing stale corpus browser pages");
    }
    fs::create_dir_all(&directory).expect("creating corpus browser directory");
    for file in render_browser() {
        let path = directory.join(file.name);
        fs::write(&path, file.content)
            .unwrap_or_else(|error| panic!("writing {}: {error}", path.display()));
        eprintln!("wrote {}", path.display());
    }
}

#[test]
fn generated_corpus_browser_is_deterministic() {
    assert_eq!(render_browser(), render_browser());
}

#[test]
fn corpus_browser_matches_committed_pages() {
    let expected = render_browser();
    let mut expected_names = expected
        .iter()
        .map(|file| file.name.clone())
        .collect::<Vec<_>>();
    expected_names.sort();
    let mut actual_names = fs::read_dir(browser_dir())
        .expect("reading docs/corpus")
        .map(|entry| {
            entry
                .expect("reading docs/corpus entry")
                .file_name()
                .into_string()
                .expect("corpus browser filename is UTF-8")
        })
        .collect::<Vec<_>>();
    actual_names.sort();
    assert_eq!(
        actual_names, expected_names,
        "docs/corpus has added, removed, or renamed pages; run `cargo test --test corpus -- --ignored generate_corpus_browser`"
    );
    for file in expected {
        let path = browser_dir().join(file.name);
        let actual = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        assert_eq!(
            actual,
            file.content,
            "{} has drifted; run `cargo test --test corpus -- --ignored generate_corpus_browser`",
            path.display()
        );
    }
}
