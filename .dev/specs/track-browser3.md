# track/browser3 — corpus browser v3: every file visible, tabs, gap panel

Read AGENTS.md first (gate convention; synchronous receipts), then
docs/corpus.md §"How this corpus is maintained (the loops)" for the
GAPS.md convention you will render. Work in the herdr worktree on branch
`track/browser3`. Keep `crates/cix/LOG.md` current (create if absent;
append-only, timestamped).

Purpose (Mathijs, 2026-08-04): the published corpus page
(mathijshenquet.nl/composix) is human-consumable documentation. Today a
reader cannot see mastodon's per-member Cixfiles, cannot tell why dozzle
is red or excalidraw is orange, and cannot compare against the
Dockerfile's context files. Fix the *rendering*; the content ledgers are
another track's job.

FENCE: track/corpusgaps runs concurrently on the content side. Do NOT
touch `docs/corpus.md`, `docs/migrate.md`, any `corpus/migrate/*/GAPS.md`,
any Cixfile/lock/receipt/check.sh (except the fetch.sh extension below).
Your domain: `crates/cix/tests/corpus.rs` (generator), `docs/corpus/`
(generated output), `corpus/migrate/fetch.sh`, new
`corpus/migrate/*/context.files` manifests, `crates/cix/LOG.md`.
Merge note: the generated output will conflict with corpusgaps at merge
time; the orchestrator resolves by regenerating — do not pre-coordinate.

## 1. Render every checked-in subordinate file

Per-case page, two columns as today (upstream left, cix right), but
complete:

- **Right (cix)**: the Cixfile (tabs if variants, §3); per-member
  Cixfiles from subdirectories (mastodon's web/redis/postgres/sidekiq/
  streaming/cleanup) each with their own aux files; compose.json;
  overlay `.nix` files (wallos php.nix); checked-in scripts and configs
  (setup.sh, start.sh, redis.conf, …). Collapsed by default (native
  `<details>`, no JS): Cixfile.lock, check.sh, receipt.md.
- **Left (upstream)**: Dockerfile(s); upstream compose files; SOURCE
  provenance rendered as text; the context schematic (§2).
- **Mastodon specifically**: the page must make tag provenance
  self-evident — one line of prose noting that compose children reference
  `corpus-mastodon-<member>:checked` tags produced by check.sh from the
  per-member Cixfiles shown below.

Existing syntax highlighting and the corpusstyle v2 canon (fonts, light/
dark palette) stay; extend the tokenizer only where a newly rendered file
type needs it (yaml/nix can render as plain `pre` — do not build new
highlighters beyond trivial reuse).

## 2. Upstream-context schematic (checked-in manifests)

`corpus/migrate/*/context/` is gitignored (fetched by fetch.sh), so the
generator cannot read it. Extend `fetch.sh` so that after a successful
fetch it also writes `corpus/migrate/<case>/context.files`: a
deterministic listing — one line per file, `<relative-path>\t<bytes>`,
sorted bytewise, directories excluded. Run the fetch for every case that
has a SOURCE and check the manifests in. If a fetch fails, record the
exact failure in your LOG, skip the manifest, and the page renders
"context not fetched" for that case — never a fabricated listing. The
generator renders the manifest as a collapsed file tree with sizes, so a
reader sees precisely which upstream files exist beyond the Dockerfile.

## 3. Translation-variant tabs

Discovery rule: `Cixfile` is the canonical (Dockerfile-faithful)
translation; a sibling `Cixfile.dissolved` (plus `Cixfile.dissolved.lock`)
is the nixpkgs-direct twin. Render as CSS-only tabs (radio-input pattern,
no JS), default tab = faithful. No twins exist in-tree yet: unit-test the
mechanism against a fixture directory in the test itself, and make sure a
case with only `Cixfile` renders exactly as a tabless page. If the
CSS-only approach fights the existing layout, a labeled side-by-side
stack is an acceptable fallback — note the choice in your LOG.

## 4. Gap panel

If `corpus/migrate/<case>/GAPS.md` exists, render it at the top of the
case page as a visually distinct panel: the `Generated:`/`Status:` header
pair styled as metadata (a stale Status gets a warning tint), the body as
lightly rendered markdown (paragraphs, bullets, inline code, links —
nothing fancier). Absent file → no panel. None exist in-tree yet:
fixture-test the rendering. This panel is the answer to "red for
inscrutable reasons" — when the ledgers land, the page explains itself.

## 5. Rot guard in CI

New non-ignored test in the same file: parse every
`corpus/migrate/**/Cixfile*` (excluding locks) with the real cix-cixfile
parser; collect and report all failures, assert none. All current corpus
Cixfiles are believed to parse — if one does not, that is a finding:
report it in your LOG and leave the test listing it as expected-failure
with a comment, do not "fix" the Cixfile (fenced).

## 6. Determinism

The browser stays a deterministic drift-checked generation: same inputs →
byte-identical output, `cargo test --test corpus -- --ignored
generate_corpus_browser` regenerates, the non-ignored drift test
compares. Regenerate and commit the output.

## Gate

Standard agent tier: `cargo fmt --check`, examples fmt, warning-denied
clippy, full workspace tests, tour regen+drift. No VM scenarios are
touched by this track. Receipts are synchronous exit statuses in your
LOG with exact repro commands.
