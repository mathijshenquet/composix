//! Deterministic side-by-side browser generated from `corpus/migrate/`.
//!
//! Run `cargo test --test corpus -- --ignored generate_corpus_browser` to update it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use cix_test_support::{assert_generated_matches, write_generated_atomically, GeneratedFile};

mod corpus_parts;

use corpus_parts::{discovery::*, highlight::*, templates::*};

const REPOSITORY_URL: &str = "https://github.com/mathijshenquet/composix/blob/main";
const PAGE_STYLE: &str = r#"
    :root { color-scheme: light dark; font-family: system-ui, sans-serif; --page: #f8fafc; --ink: #182230; --muted: #667085; --line: #d0d5dd; --panel: #fff; --code: #f3f5f7; --link: #1769aa; --directive: #7856a8; --interpolation: #9a4f00; --comment: #68717d; --string: #39704a; --number: #a23b51; --key: #2f6598; --literal: #8c3f72; --heredoc: #344054; }
    @media (prefers-color-scheme: dark) { :root { --page: #11161d; --ink: #e7ebf0; --muted: #9da8b8; --line: #3a4553; --panel: #171e27; --code: #0d1218; --link: #78b7ef; --directive: #c4a7e7; --interpolation: #e8ae73; --comment: #8995a5; --string: #8fc89f; --number: #e58a9d; --key: #82b9e8; --literal: #d79bc2; --heredoc: #c2cad5; } }
    * { box-sizing: border-box; }
    body { max-width: 92rem; margin: 0 auto; padding: 1rem; line-height: 1.42; background: var(--page); color: var(--ink); }
    a { color: var(--link); }
    h1 { margin: .45rem 0 .55rem; font-size: 1.75rem; }
    h2 { margin: 0; padding: .65rem .75rem 0; font-size: 1.1rem; }
    p { margin: .45rem 0; }
    .crumbs, .status, .source { color: var(--muted); font-size: .92rem; }
    .artifacts { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: .75rem; margin-top: .8rem; align-items: start; }
    .column { min-width: 0; overflow-x: auto; border: 1px solid var(--line); border-radius: .45rem; background: var(--panel); padding-bottom: .75rem; }
    .file { min-width: 100%; width: max-content; max-width: none; margin-top: .65rem; padding: 0 .75rem; }
    .file h3 { position: sticky; left: .75rem; width: fit-content; margin: 0 0 .3rem; font-size: .84rem; font-weight: 650; color: var(--muted); }
    .file summary { position: sticky; left: .75rem; width: fit-content; cursor: pointer; color: var(--muted); font-size: .84rem; font-weight: 650; }
    pre { min-width: 100%; width: max-content; margin: 0; padding: .7rem .8rem; border-radius: .3rem; background: var(--code); color: var(--ink); white-space: pre; tab-size: 2; font-size: 13px; line-height: 1.42; }
    code { font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace; }
    .tok-directive { color: var(--directive); font-weight: 650; }
    .tok-interpolation { color: var(--interpolation); font-weight: 600; }
    .tok-comment { color: var(--comment); font-style: italic; }
    .tok-string { color: var(--string); }
    .tok-number { color: var(--number); }
    .tok-key { color: var(--key); }
    .tok-literal { color: var(--literal); }
    .tok-heredoc { color: var(--heredoc); }
    table { width: 100%; border-collapse: collapse; margin-top: .8rem; background: var(--panel); }
    th, td { padding: .48rem .55rem; border-bottom: 1px solid var(--line); text-align: left; vertical-align: top; }
    th { white-space: nowrap; }
    .context, .gap-panel { margin: .8rem 0; padding: .65rem .75rem; border: 1px solid var(--line); border-radius: .45rem; background: var(--panel); }
    .context summary { cursor: pointer; font-weight: 650; }
    .file-tree { margin: .55rem 0 0; padding-left: 1.35rem; font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace; font-size: .84rem; }
    .file-tree span { color: var(--muted); }
    .gap-panel { border-left: .35rem solid var(--interpolation); }
    .gap-panel.stale { border-left-color: var(--number); }
    .gap-meta { color: var(--muted); font-size: .9rem; }
    .gap-panel ul { margin: .45rem 0; padding-left: 1.35rem; }
    .variant-tabs { margin-top: .65rem; padding: 0 .75rem; }
    .variant-tabs input { position: absolute; opacity: 0; pointer-events: none; }
    .variant-tabs label { display: inline-block; margin-right: .3rem; padding: .25rem .5rem; border: 1px solid var(--line); border-bottom: 0; border-radius: .3rem .3rem 0 0; cursor: pointer; color: var(--muted); font-size: .84rem; }
    .variant-tabs input:checked + label { background: var(--code); color: var(--ink); font-weight: 650; }
    .variant-panel { display: none; }
    .variant-panel .file { margin-top: .3rem; padding: 0; }
    .variant-panel .file h3 { display: none; }
    .variant-tabs input:checked + label + input + label + .variant-panels .variant-panel.faithful { display: block; }
    .variant-tabs input + label + input:checked + label + .variant-panels .variant-panel.dissolved { display: block; }
    footer { margin-top: 1.25rem; color: var(--muted); font-size: .85rem; }
    @media (max-width: 46rem) { .artifacts { grid-template-columns: minmax(0, 1fr); } body { padding: .75rem; } }
"#;

#[derive(Clone)]
struct Artifact {
    name: String,
    content: String,
    collapsed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    Cixfile,
    Dockerfile,
    Json,
    Nix,
    Plain,
    Yaml,
}

#[derive(Clone)]
struct LedgerGrade {
    receipt_status: String,
    ribbon: String,
    evidence: String,
}

struct Case {
    axis: String,
    name: String,
    upstream: Vec<Artifact>,
    cix: Vec<Artifact>,
    grade: LedgerGrade,
    context_files: Option<Vec<ContextFile>>,
    gap: Option<GapPanel>,
}

struct ContextFile {
    path: String,
    bytes: u64,
}

struct GapPanel {
    generated: String,
    status: String,
    body: String,
}

fn render_browser() -> Vec<GeneratedFile> {
    let cases = load_cases();
    let mut files = Vec::with_capacity(cases.len() + 1);
    files.push(GeneratedFile {
        name: "index.html".to_owned(),
        content: render_index(&cases),
    });
    files.extend(cases.iter().map(|case| GeneratedFile {
        name: format!("{}-{}.html", case.axis, case.name),
        content: render_case(case),
    }));
    files
}

#[test]
#[ignore = "run explicitly to update docs/corpus/"]
fn generate_corpus_browser() {
    let directory = browser_dir();
    write_generated_atomically(&directory, &render_browser())
        .unwrap_or_else(|error| panic!("publishing {}: {error:#}", directory.display()));
}

#[test]
fn generated_corpus_browser_is_deterministic() {
    assert_eq!(render_browser(), render_browser());
}

#[test]
fn corpus_browser_matches_committed_pages() {
    let expected = render_browser();
    assert_generated_matches(&browser_dir(), &expected).unwrap_or_else(|error| {
        panic!("docs/corpus drift; run `cargo test --test corpus -- --ignored generate_corpus_browser`: {error:#}")
    });
}

#[test]
fn every_checked_in_corpus_cixfile_parses() {
    let failures = corpus_files(&corpus_dir())
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("Cixfile") && !name.ends_with(".lock"))
        })
        .filter_map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            cix_cixfile::parse(&source)
                .err()
                .map(|error| format!("{}: {error:#}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "checked-in corpus Cixfiles that fail the real parser:\n{}",
        failures.join("\n")
    );
}

#[test]
fn cixfile_variants_render_as_faithful_default_tabs_and_solo_files_stay_tabless() {
    let faithful = Artifact {
        name: "Cixfile".to_owned(),
        content: "SERVICE web\n".to_owned(),
        collapsed: false,
    };
    let dissolved = Artifact {
        name: "Cixfile.dissolved".to_owned(),
        content: "SERVICE web\n".to_owned(),
        collapsed: false,
    };
    let variants = render_cix_artifacts("fixture", &[faithful.clone(), dissolved]);
    assert!(variants.contains("Dockerfile-faithful"));
    assert!(variants.contains("nixpkgs-direct"));
    assert!(variants
        .contains("id=\"tabs-fixture-0-faithful\" name=\"tabs-fixture-0\" type=\"radio\" checked"));

    let solo = render_cix_artifacts("fixture", &[faithful]);
    assert!(!solo.contains("type=\"radio\""));
    assert!(solo.contains("<h3>Cixfile</h3>"));
}

#[test]
fn gap_panel_renders_metadata_stale_warning_and_limited_markdown() {
    let panel = GapPanel {
        generated: "migrate.md@abc · test model · 2026-08-04".to_owned(),
        status: "stale — regenerate with CIP-99".to_owned(),
        body: "A [link](https://example.invalid) with `code`.\n\n- first item\n- second item"
            .to_owned(),
    };
    let rendered = render_gap_panel(Some(&panel));
    assert!(rendered.contains("gap-panel stale"));
    assert!(rendered.contains("<strong>Generated:</strong> migrate.md@abc"));
    assert!(rendered.contains("<a href=\"https://example.invalid\">link</a>"));
    assert!(rendered.contains("<code>code</code>"));
    assert!(rendered.contains("<li>first item</li>"));
}

#[test]
fn cixfile_highlighting_tracks_directives_comments_interpolation_and_heredocs() {
    let source = "# context\nSERVICE web\n  FILE /etc/app.conf <<CONF\npath = ${pkgs.app}\nliteral = $${kept}\nCONF\n";
    let highlighted = highlight(source, Language::Cixfile);
    assert!(highlighted.contains("<span class=\"tok-comment\"># context</span>"));
    assert!(highlighted.contains("<span class=\"tok-directive\">SERVICE</span> web"));
    assert!(highlighted.contains("<span class=\"tok-directive\">FILE</span>"));
    assert!(highlighted.contains(
        "<span class=\"tok-heredoc\">path = </span><span class=\"tok-interpolation\">${pkgs.app}</span>"
    ));
    assert!(highlighted.contains("<span class=\"tok-heredoc\">literal = $${kept}</span>"));
    assert!(highlighted.contains("<span class=\"tok-directive\">CONF</span>"));
}

#[test]
fn supported_artifact_languages_are_escaped_and_highlighted() {
    let cases = [
        (
            Language::Dockerfile,
            "# note\nFROM example\nRUN echo '<unsafe>'\n",
            "tok-directive",
        ),
        (
            Language::Nix,
            "final: prev: { enabled = true; package = \"<unsafe>\"; }",
            "tok-string",
        ),
        (
            Language::Json,
            "{\"enabled\":true,\"count\":2,\"text\":\"<unsafe>\"}",
            "tok-key",
        ),
        (
            Language::Yaml,
            "enabled: true\ntext: \"<unsafe>\" # note\n",
            "tok-key",
        ),
    ];
    for (language, source, expected_class) in cases {
        let highlighted = highlight(source, language);
        assert!(highlighted.contains(expected_class), "{language:?}");
        assert!(highlighted.contains("&lt;unsafe&gt;"), "{language:?}");
        assert!(!highlighted.contains("<unsafe>"), "{language:?}");
    }
}
