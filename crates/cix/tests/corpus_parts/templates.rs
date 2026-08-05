use super::super::*;
pub(crate) fn page_start(title: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>{PAGE_STYLE}  </style>\n</head>\n<body>\n",
        html_escape(title)
    )
}

// Rendered-page clarity guard: giant generated artifacts (node-tree
// locks) made docker-it-tools.html 100.49 MB, past GitHub's 100 MB
// push limit and past any human use. Render the head and say so.
const ARTIFACT_RENDER_LINE_CAP: usize = 10_000;

pub(crate) fn render_artifact(artifact: &Artifact) -> String {
    let language = language_for(&artifact.name);
    let total_lines = artifact.content.lines().count();
    let capped;
    let body = if total_lines > ARTIFACT_RENDER_LINE_CAP {
        capped = artifact
            .content
            .lines()
            .take(ARTIFACT_RENDER_LINE_CAP)
            .collect::<Vec<_>>()
            .join("\n");
        &capped
    } else {
        &artifact.content
    };
    let mut content = highlight(body, language);
    if total_lines > ARTIFACT_RENDER_LINE_CAP {
        write!(
            content,
            "\n… truncated for rendering: {} of {} lines shown; the full file lives in the repository.",
            ARTIFACT_RENDER_LINE_CAP, total_lines
        )
        .expect("appending truncation note");
    }
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

pub(crate) fn render_artifacts(artifacts: &[Artifact]) -> String {
    artifacts.iter().map(render_artifact).collect()
}

pub(crate) fn render_cix_artifacts(case_name: &str, artifacts: &[Artifact]) -> String {
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

pub(crate) fn render_context_files(context_files: Option<&[ContextFile]>) -> String {
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

pub(crate) fn render_inline_markdown(source: &str) -> String {
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
                output.push('`');
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

pub(crate) fn render_gap_markdown(source: &str) -> String {
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

pub(crate) fn render_gap_panel(gap: Option<&GapPanel>) -> String {
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

pub(crate) fn ribbon_and_evidence(case: &Case) -> (&str, &str) {
    (case.grade.ribbon.as_str(), case.grade.evidence.as_str())
}

pub(crate) fn render_case(case: &Case) -> String {
    let title = format!("{} / {} — migration corpus", case.axis, case.name);
    let mut page = page_start(&title);
    let (ribbon, evidence) = ribbon_and_evidence(case);
    let receipt_url = format!(
        "{REPOSITORY_URL}/corpus/migrate/{}/{}/receipt.md",
        case.axis, case.name
    );
    let source_url = format!(
        "{REPOSITORY_URL}/corpus/migrate/{}/{}/SOURCE",
        case.axis, case.name
    );
    let mastodon_note = (case.axis == "docker" && case.name == "mastodon").then_some(
        "  <p class=\"status\"><strong>Mastodon tag provenance:</strong> compose children reference <code>corpus-mastodon-&lt;member&gt;:checked</code> tags produced by <code>check.sh</code> from the per-member Cixfiles shown below.</p>\n",
    );
    write!(
        page,
        "  <nav class=\"crumbs\"><a href=\"index.html\">Corpus browser</a> / {axis} / {name}</nav>\n  <h1>{axis} / {name}</h1>\n  <p class=\"status\"><strong>docs/corpus.md ribbon:</strong> {ribbon} · <strong>Evidence:</strong> {evidence}</p>\n  <p class=\"status\"><strong>Receipt status:</strong> {receipt_status} · <a href=\"{receipt_url}\">Full migration receipt</a></p>\n  <p class=\"source\"><a href=\"{source_url}\">Pinned upstream source notes</a></p>\n{mastodon_note}{gap}{context}  <main class=\"artifacts\">\n    <section class=\"column\">\n      <h2>Upstream</h2>\n{upstream}    </section>\n    <section class=\"column\">\n      <h2>composix</h2>\n{cix}    </section>\n  </main>\n  <footer>Generated from <code>corpus/migrate/{axis}/{name}</code>. Do not edit this page by hand.</footer>\n</body>\n</html>\n",
        axis = html_escape(&case.axis),
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

pub(crate) fn render_index(cases: &[Case]) -> String {
    let mut page = page_start("Migration corpus browser — composix");
    page.push_str(
        "  <nav class=\"crumbs\"><a href=\"../index.html\">composix docs</a> / corpus</nav>\n  <h1>Migration corpus</h1>\n  <p>One living corpus, generated directly from the checked-in upstream and composix artifacts. Each case opens as a two-column comparison; its receipt records what was actually verified and what remains open.</p>\n  <p class=\"status\"><strong>Do not edit these pages.</strong> Regenerate with <code>cargo test --test corpus -- --ignored generate_corpus_browser</code>. Normal tests reject drift.</p>\n",
    );
    for axis in ["docker", "k8s"] {
        let heading = if axis == "docker" {
            "Docker cases"
        } else {
            "Kubernetes cases"
        };
        write!(
            page,
            "  <h2>{heading}</h2>\n  <table>\n    <thead><tr><th>Case</th><th>docs/corpus.md ribbon</th><th>Evidence class</th></tr></thead>\n    <tbody>\n"
        )
        .expect("rendering corpus axis heading");
        for case in cases.iter().filter(|case| case.axis == axis) {
            let (ribbon, evidence) = ribbon_and_evidence(case);
            writeln!(
                page,
                "      <tr><td><a href=\"{}-{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>",
                html_escape(&case.axis),
                html_escape(&case.name),
                html_escape(&case.name),
                html_escape(ribbon),
                html_escape(evidence)
            )
            .expect("rendering corpus index row");
        }
    }
    page.push_str(
        "    </tbody>\n  </table>\n  <footer>Generated from <code>corpus/migrate/</code> and the ribbons in <code>docs/corpus.md</code>.</footer>\n</body>\n</html>\n",
    );
    page
}
