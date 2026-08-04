//! Deterministic side-by-side browser generated from `corpus/migrate/`.
//!
//! Run `cargo test --test corpus -- --ignored generate_corpus_browser` to update it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, PartialEq, Eq)]
struct GeneratedFile {
    name: String,
    content: String,
}

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

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_dir() -> PathBuf {
    repository_root().join("corpus/migrate")
}

fn browser_dir() -> PathBuf {
    repository_root().join("docs/corpus")
}

fn corpus_files(directory: &Path) -> Vec<PathBuf> {
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

fn relative_name(directory: &Path, path: &Path) -> String {
    path.strip_prefix(directory)
        .expect("corpus artifact remains under its case directory")
        .to_string_lossy()
        .replace('\\', "/")
}

fn read_artifact(directory: &Path, path: &Path, collapsed: bool) -> Artifact {
    Artifact {
        name: relative_name(directory, path),
        content: fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
        collapsed,
    }
}

fn is_upstream_artifact(name: &str) -> bool {
    name == "Dockerfile"
        || name.starts_with("upstream-")
        || name == "SOURCE"
        || name == "context.files"
}

fn is_collapsed_cix_artifact(name: &str) -> bool {
    name == "check.sh"
        || name == "receipt.md"
        || name.ends_with("Cixfile.lock")
        || name.ends_with("Cixfile.dissolved.lock")
}

fn read_upstream_artifacts(directory: &Path) -> Vec<Artifact> {
    corpus_files(directory)
        .into_iter()
        .filter(|path| is_upstream_artifact(&relative_name(directory, path)))
        .filter(|path| relative_name(directory, path) != "context.files")
        .map(|path| read_artifact(directory, &path, false))
        .collect()
}

fn read_cix_artifacts(directory: &Path) -> Vec<Artifact> {
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

fn read_context_files(directory: &Path) -> Option<Vec<ContextFile>> {
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

fn read_gap_panel(directory: &Path) -> Option<GapPanel> {
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
                context_files: read_context_files(&directory),
                gap: read_gap_panel(&directory),
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

fn language_for(name: &str) -> Language {
    match name {
        "Cixfile" => Language::Cixfile,
        "Dockerfile" => Language::Dockerfile,
        _ if name.ends_with(".json") => Language::Json,
        _ if name.ends_with(".nix") => Language::Nix,
        _ if name.ends_with(".yaml") || name.ends_with(".yml") => Language::Yaml,
        _ => Language::Plain,
    }
}

fn push_span(output: &mut String, class: &str, source: &str) {
    write!(
        output,
        "<span class=\"{class}\">{}</span>",
        html_escape(source)
    )
    .expect("rendering highlighted span");
}

fn push_styled(output: &mut String, class: Option<&str>, source: &str) {
    if source.is_empty() {
        return;
    }
    if let Some(class) = class {
        push_span(output, class, source);
    } else {
        output.push_str(&html_escape(source));
    }
}

fn highlight_interpolations(source: &str, base_class: Option<&str>) -> String {
    let mut output = String::new();
    let mut emitted = 0;
    let mut search_from = 0;
    while let Some(relative_start) = source[search_from..].find("${") {
        let start = search_from + relative_start;
        if start > 0 && source.as_bytes()[start - 1] == b'$' {
            search_from = start + 2;
            continue;
        }
        let Some(relative_end) = source[start + 2..].find('}') else {
            break;
        };
        let end = start + 2 + relative_end + 1;
        push_styled(&mut output, base_class, &source[emitted..start]);
        push_span(&mut output, "tok-interpolation", &source[start..end]);
        emitted = end;
        search_from = end;
    }
    push_styled(&mut output, base_class, &source[emitted..]);
    output
}

fn directive_line(source: &str, body_class: Option<&str>) -> String {
    let indent_end = source.len() - source.trim_start().len();
    let trimmed = &source[indent_end..];
    let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let mut output = html_escape(&source[..indent_end]);
    push_span(&mut output, "tok-directive", &trimmed[..directive_end]);
    output.push_str(&highlight_interpolations(
        &trimmed[directive_end..],
        body_class,
    ));
    output
}

fn heredoc_marker(source: &str) -> Option<String> {
    let start = source.find("<<")? + 2;
    let marker = source[start..]
        .split_whitespace()
        .next()
        .unwrap_or_default();
    (!marker.is_empty()).then(|| marker.trim_matches(['\'', '"']).to_owned())
}

fn highlight_cixfile(source: &str) -> String {
    let mut output = String::new();
    let mut heredoc = None::<String>;
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let trimmed = line.trim_start();
        if let Some(marker) = heredoc.as_deref() {
            if trimmed == marker {
                output.push_str(&directive_line(line, None));
                heredoc = None;
            } else {
                output.push_str(&highlight_interpolations(line, Some("tok-heredoc")));
            }
        } else if trimmed.starts_with('#') {
            let indent_end = line.len() - trimmed.len();
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-comment", trimmed);
        } else if trimmed.is_empty() {
            output.push_str(&html_escape(line));
        } else {
            output.push_str(&directive_line(line, None));
            heredoc = heredoc_marker(line);
        }
        output.push_str(newline);
    }
    output
}

fn quoted_end(source: &str, start: usize, quote: u8) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
        } else if bytes[cursor] == quote {
            return cursor + 1;
        } else {
            cursor += 1;
        }
    }
    bytes.len()
}

fn highlight_shellish(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let bytes = source.as_bytes();
        if matches!(bytes[cursor], b'\'' | b'"') {
            let end = quoted_end(source, cursor, bytes[cursor]);
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if source[cursor..].starts_with("${") && (cursor == 0 || bytes[cursor - 1] != b'$') {
            let end = source[cursor + 2..]
                .find('}')
                .map_or(source.len(), |end| cursor + 2 + end + 1);
            push_span(&mut output, "tok-interpolation", &source[cursor..end]);
            cursor = end;
        } else {
            let character = source[cursor..].chars().next().expect("source character");
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

fn highlight_dockerfile(source: &str) -> String {
    let mut output = String::new();
    let mut continued = false;
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let indent_end = line.len() - trimmed.len();
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-comment", trimmed);
        } else if !continued && !trimmed.is_empty() {
            let indent_end = line.len() - trimmed.len();
            let directive_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            output.push_str(&html_escape(&line[..indent_end]));
            push_span(&mut output, "tok-directive", &trimmed[..directive_end]);
            output.push_str(&highlight_shellish(&trimmed[directive_end..]));
        } else {
            output.push_str(&highlight_shellish(line));
        }
        continued = !trimmed.starts_with('#') && line.trim_end().ends_with('\\');
        output.push_str(newline);
    }
    output
}

fn identifier_end(source: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, character) in source[start..].char_indices() {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            end = start + offset + character.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn highlight_nix(source: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "assert", "else", "if", "in", "inherit", "let", "or", "rec", "then", "with",
    ];
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        if source[cursor..].starts_with("/*") {
            let end = source[cursor + 2..]
                .find("*/")
                .map_or(source.len(), |end| cursor + 2 + end + 2);
            push_span(&mut output, "tok-comment", &source[cursor..end]);
            cursor = end;
        } else if source.as_bytes()[cursor] == b'#' {
            let end = source[cursor..]
                .find('\n')
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-comment", &source[cursor..end]);
            cursor = end;
        } else if source.as_bytes()[cursor] == b'"' {
            let end = quoted_end(source, cursor, b'"');
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if source[cursor..].starts_with("${") {
            let end = source[cursor + 2..]
                .find('}')
                .map_or(source.len(), |end| cursor + 2 + end + 1);
            push_span(&mut output, "tok-interpolation", &source[cursor..end]);
            cursor = end;
        } else {
            let character = source[cursor..].chars().next().expect("source character");
            if character.is_alphabetic() || character == '_' {
                let end = identifier_end(source, cursor);
                let word = &source[cursor..end];
                if KEYWORDS.contains(&word) {
                    push_span(&mut output, "tok-directive", word);
                } else {
                    output.push_str(&html_escape(word));
                }
                cursor = end;
            } else if character.is_ascii_digit() {
                let end = source[cursor..]
                    .find(|value: char| !value.is_ascii_digit() && value != '.')
                    .map_or(source.len(), |end| cursor + end);
                push_span(&mut output, "tok-number", &source[cursor..end]);
                cursor = end;
            } else {
                output.push_str(&html_escape(&character.to_string()));
                cursor += character.len_utf8();
            }
        }
    }
    output
}

fn highlight_json(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let character = source[cursor..].chars().next().expect("source character");
        if character == '"' {
            let end = quoted_end(source, cursor, b'"');
            let is_key = source[end..]
                .chars()
                .find(|character| !character.is_whitespace())
                == Some(':');
            push_span(
                &mut output,
                if is_key { "tok-key" } else { "tok-string" },
                &source[cursor..end],
            );
            cursor = end;
        } else if character.is_ascii_digit() || character == '-' {
            let end = source[cursor..]
                .find(|value: char| {
                    !value.is_ascii_digit() && !matches!(value, '-' | '+' | '.' | 'e' | 'E')
                })
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-number", &source[cursor..end]);
            cursor = end;
        } else if character.is_alphabetic() {
            let end = identifier_end(source, cursor);
            push_span(&mut output, "tok-literal", &source[cursor..end]);
            cursor = end;
        } else {
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

fn yaml_comment_start(source: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' && quote == Some('"') {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if character == '#' && quote.is_none() {
            return Some(index);
        }
    }
    None
}

fn yaml_key_end(source: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in source.char_indices() {
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if character == ':' && quote.is_none() {
            return Some(index);
        }
    }
    None
}

fn highlight_yaml_value(source: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let character = source[cursor..].chars().next().expect("source character");
        if matches!(character, '\'' | '"') {
            let end = quoted_end(source, cursor, character as u8);
            push_span(&mut output, "tok-string", &source[cursor..end]);
            cursor = end;
        } else if character.is_ascii_digit() || character == '-' {
            let end = source[cursor..]
                .find(|value: char| !value.is_ascii_digit() && !matches!(value, '-' | '.'))
                .map_or(source.len(), |end| cursor + end);
            push_span(&mut output, "tok-number", &source[cursor..end]);
            cursor = end;
        } else if character.is_alphabetic() {
            let end = identifier_end(source, cursor);
            let word = &source[cursor..end];
            if matches!(word, "false" | "null" | "true" | "yes" | "no") {
                push_span(&mut output, "tok-literal", word);
            } else {
                output.push_str(&html_escape(word));
            }
            cursor = end;
        } else {
            output.push_str(&html_escape(&character.to_string()));
            cursor += character.len_utf8();
        }
    }
    output
}

fn highlight_yaml(source: &str) -> String {
    let mut output = String::new();
    for physical_line in source.split_inclusive('\n') {
        let (line, newline) = physical_line
            .strip_suffix('\n')
            .map_or((physical_line, ""), |line| (line, "\n"));
        let comment_start = yaml_comment_start(line).unwrap_or(line.len());
        let code = &line[..comment_start];
        let indent_end = code.len() - code.trim_start().len();
        let trimmed = &code[indent_end..];
        output.push_str(&html_escape(&code[..indent_end]));
        let key_offset = trimmed.strip_prefix("- ").map_or(0, |_| 2);
        output.push_str(&html_escape(&trimmed[..key_offset]));
        let rest = &trimmed[key_offset..];
        if let Some(key_end) = yaml_key_end(rest) {
            push_span(&mut output, "tok-key", &rest[..key_end]);
            output.push(':');
            output.push_str(&highlight_yaml_value(&rest[key_end + 1..]));
        } else {
            output.push_str(&highlight_yaml_value(rest));
        }
        if comment_start < line.len() {
            push_span(&mut output, "tok-comment", &line[comment_start..]);
        }
        output.push_str(newline);
    }
    output
}

fn highlight(source: &str, language: Language) -> String {
    match language {
        Language::Cixfile => highlight_cixfile(source),
        Language::Dockerfile => highlight_dockerfile(source),
        Language::Json => highlight_json(source),
        Language::Nix => highlight_nix(source),
        Language::Plain => html_escape(source),
        Language::Yaml => highlight_yaml(source),
    }
}

fn page_start(title: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>{PAGE_STYLE}  </style>\n</head>\n<body>\n",
        html_escape(title)
    )
}

fn render_artifact(artifact: &Artifact) -> String {
    let language = language_for(&artifact.name);
    let content = highlight(&artifact.content, language);
    if artifact.collapsed {
        format!(
            "      <details class=\"file\">\n        <summary>{}</summary>\n        <pre><code>{}</code></pre>\n      </details>\n",
            html_escape(&artifact.name),
            content
        )
    } else {
        format!(
            "      <section class=\"file\">\n        <h3>{}</h3>\n        <pre><code>{}</code></pre>\n      </section>\n",
            html_escape(&artifact.name),
            content
        )
    }
}

fn render_artifacts(artifacts: &[Artifact]) -> String {
    artifacts.iter().map(render_artifact).collect()
}

fn render_cix_artifacts(case_name: &str, artifacts: &[Artifact]) -> String {
    let mut output = String::new();
    let mut tab_number = 0;
    for artifact in artifacts {
        if artifact.name.rsplit('/').next() != Some("Cixfile") {
            if !artifact.name.ends_with("Cixfile.dissolved") {
                output.push_str(&render_artifact(artifact));
            }
            continue;
        }
        let dissolved_name = format!("{}.dissolved", artifact.name);
        let Some(dissolved) = artifacts
            .iter()
            .find(|candidate| candidate.name == dissolved_name)
        else {
            output.push_str(&render_artifact(artifact));
            continue;
        };
        let id = format!("tabs-{case_name}-{tab_number}");
        tab_number += 1;
        write!(
            output,
            "      <section class=\"variant-tabs\">\n        <input id=\"{id}-faithful\" name=\"{id}\" type=\"radio\" checked>\n        <label for=\"{id}-faithful\">Dockerfile-faithful</label>\n        <input id=\"{id}-dissolved\" name=\"{id}\" type=\"radio\">\n        <label for=\"{id}-dissolved\">nixpkgs-direct</label>\n        <div class=\"variant-panels\">\n          <div class=\"variant-panel faithful\">{}          </div>\n          <div class=\"variant-panel dissolved\">{}          </div>\n        </div>\n      </section>\n",
            render_artifact(artifact),
            render_artifact(dissolved),
        )
        .expect("rendering Cixfile variants");
    }
    output
}

fn render_context_files(context_files: Option<&[ContextFile]>) -> String {
    let Some(context_files) = context_files else {
        return "  <p class=\"context\"><strong>Upstream build context:</strong> context not fetched.</p>\n".to_owned();
    };
    let total_bytes = context_files.iter().map(|file| file.bytes).sum::<u64>();
    let mut output = format!(
        "  <details class=\"context\">\n    <summary>Upstream build context: {} files, {} bytes</summary>\n    <ul class=\"file-tree\">\n",
        context_files.len(), total_bytes
    );
    for file in context_files {
        writeln!(
            output,
            "      <li><code>{}</code> <span>{} bytes</span></li>",
            html_escape(&file.path),
            file.bytes
        )
        .expect("rendering context file");
    }
    output.push_str("    </ul>\n  </details>\n");
    output
}

fn render_inline_markdown(source: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        let code_at = rest.find('`');
        let link_at = rest.find('[');
        let next = match (code_at, link_at) {
            (Some(code), Some(link)) => code.min(link),
            (Some(code), None) => code,
            (None, Some(link)) => link,
            (None, None) => {
                output.push_str(&html_escape(rest));
                break;
            }
        };
        output.push_str(&html_escape(&rest[..next]));
        rest = &rest[next..];
        if let Some(after_open) = rest.strip_prefix('`') {
            if let Some(end) = after_open.find('`') {
                write!(output, "<code>{}</code>", html_escape(&after_open[..end]))
                    .expect("rendering inline code");
                rest = &after_open[end + 1..];
            } else {
                output.push_str("`");
                rest = after_open;
            }
        } else if let Some(after_open) = rest.strip_prefix('[') {
            if let Some((label, after_label)) = after_open.split_once("](") {
                if let Some((url, after_url)) = after_label.split_once(')') {
                    write!(
                        output,
                        "<a href=\"{}\">{}</a>",
                        html_escape(url),
                        html_escape(label)
                    )
                    .expect("rendering inline link");
                    rest = after_url;
                } else {
                    output.push('[');
                    rest = after_open;
                }
            } else {
                output.push('[');
                rest = after_open;
            }
        }
    }
    output
}

fn render_gap_markdown(source: &str) -> String {
    let mut output = String::new();
    let mut paragraph = Vec::new();
    let mut list_open = false;
    let flush_paragraph = |output: &mut String, paragraph: &mut Vec<&str>| {
        if !paragraph.is_empty() {
            writeln!(
                output,
                "    <p>{}</p>",
                render_inline_markdown(&paragraph.join(" "))
            )
            .expect("rendering gap paragraph");
            paragraph.clear();
        }
    };
    for line in source.lines() {
        let bullet = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "));
        if let Some(bullet) = bullet {
            flush_paragraph(&mut output, &mut paragraph);
            if !list_open {
                output.push_str("    <ul>\n");
                list_open = true;
            }
            writeln!(output, "      <li>{}</li>", render_inline_markdown(bullet))
                .expect("rendering gap bullet");
        } else if line.trim().is_empty() {
            flush_paragraph(&mut output, &mut paragraph);
            if list_open {
                output.push_str("    </ul>\n");
                list_open = false;
            }
        } else {
            if list_open {
                output.push_str("    </ul>\n");
                list_open = false;
            }
            paragraph.push(line.trim());
        }
    }
    flush_paragraph(&mut output, &mut paragraph);
    if list_open {
        output.push_str("    </ul>\n");
    }
    output
}

fn render_gap_panel(gap: Option<&GapPanel>) -> String {
    let Some(gap) = gap else {
        return String::new();
    };
    let stale = gap.status.starts_with("stale");
    format!(
        "  <aside class=\"gap-panel{}\">\n    <h2>Migration gaps</h2>\n    <p class=\"gap-meta\"><strong>Generated:</strong> {} · <strong>Status:</strong> {}</p>\n{}  </aside>\n",
        if stale { " stale" } else { "" },
        html_escape(&gap.generated),
        html_escape(&gap.status),
        render_gap_markdown(&gap.body),
    )
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
    let mastodon_note = (case.name == "mastodon").then_some(
        "  <p class=\"status\"><strong>Mastodon tag provenance:</strong> compose children reference <code>corpus-mastodon-&lt;member&gt;:checked</code> tags produced by <code>check.sh</code> from the per-member Cixfiles shown below.</p>\n",
    );
    write!(
        page,
        "  <nav class=\"crumbs\"><a href=\"index.html\">Corpus browser</a> / {name}</nav>\n  <h1>{name}</h1>\n  <p class=\"status\"><strong>docs/corpus.md ribbon:</strong> {ribbon} · <strong>Evidence:</strong> {evidence}</p>\n  <p class=\"status\"><strong>Receipt status:</strong> {receipt_status} · <a href=\"{receipt_url}\">Full migration receipt</a></p>\n  <p class=\"source\"><a href=\"{source_url}\">Pinned upstream source notes</a></p>\n{mastodon_note}{gap}{context}  <main class=\"artifacts\">\n    <section class=\"column\">\n      <h2>Upstream</h2>\n{upstream}    </section>\n    <section class=\"column\">\n      <h2>composix</h2>\n{cix}    </section>\n  </main>\n  <footer>Generated from <code>corpus/migrate/{name}</code>. Do not edit this page by hand.</footer>\n</body>\n</html>\n",
        name = html_escape(&case.name),
        ribbon = html_escape(ribbon),
        evidence = html_escape(evidence),
        receipt_status = html_escape(&case.grade.receipt_status),
        receipt_url = html_escape(&receipt_url),
        source_url = html_escape(&source_url),
        mastodon_note = mastodon_note.unwrap_or_default(),
        gap = render_gap_panel(case.gap.as_ref()),
        context = render_context_files(case.context_files.as_deref()),
        upstream = render_artifacts(&case.upstream),
        cix = render_cix_artifacts(&case.name, &case.cix),
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
