# track/corpusweb — one living migrate corpus + side-by-side website view

Read AGENTS.md first. Mathijs's asks (2026-08-02, verbatim intent):
corpus/regrade/ is confusing — no historical path-dependence; ONE
up-to-date migrate corpus with a clear overview of open gaps; and build
it into the website so a Dockerfile and its Cixfile are easy to view
side by side. Work in `.worktrees/corpusweb` on branch
`track/corpusweb`. Keep `crates/cix-cixfile/LOG.md` current.

1. **Fold the split**: move `corpus/regrade/renovate` into
   `corpus/migrate/renovate` (matching the per-case layout: SOURCE,
   check.sh where feasible, receipt.md); delete `corpus/regrade/`.
   Update every live reference (docs/corpus.md evidence pointers;
   historical LOG entries stay as written). There is exactly one
   corpus from now on: `corpus/migrate/`.
2. **Open-gaps overview**: docs/corpus.md stays the ledger; give it
   (or verify it has) one clear section that answers "what is still
   open" at a glance — the demand ranking with, per demand: status
   (met / designed-unbuilt with CIP number / refused), and the rows it
   blocks. No new doc files; sharpen the existing one.
3. **Website: side-by-side corpus browser.** Generate, for every
   `corpus/migrate/<case>` that has cix artifacts, a page showing the
   upstream artifact (Dockerfile and/or upstream compose) and the cix
   artifacts (Cixfile, compose.json when present) side by side —
   two-column layout, horizontal scroll inside columns, plus the
   receipt status line and a link to the receipt. An index page lists
   all cases with their docs/corpus.md ribbon + evidence class and
   links; docs/index.md links the corpus browser. Constraints:
   - Generated into `docs/corpus/` (GitHub Pages serves docs/; raw
     .html passes through Jekyll untouched — self-contained pages,
     inline CSS, no external assets).
   - Generation is deterministic and drift-checked in the test suite,
     tour-harness style: a `cargo test -- --ignored generate_*` writes
     the pages, a normal test asserts zero drift against the committed
     output.
   - Source of truth is the corpus files themselves; never hand-edit
     the generated pages.
4. **Docs**: docs/corpus.md status header points at the browser;
   README docs list mentions it.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green. NOTE: track/devfix runs concurrently and
regenerates docs/tour + touches crates/cix-run — stay out of both;
your generator lives beside the tour harness, not inside it.
