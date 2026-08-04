# track/corpusgaps — per-case gap ledgers, ribbon honesty, prompt addenda

Read AGENTS.md first (gate convention; synchronous receipts), then
docs/corpus.md §"How this corpus is maintained (the loops)" — you are
building that section's central artifact. Work in the herdr worktree on
branch `track/corpusgaps`. Keep `corpus/migrate/LOG.md` current
(append-only, timestamped).

FENCE: track/browser3 runs concurrently on the browser generator. Do NOT
touch `crates/**`, `docs/corpus/` (generated output), or any
`corpus/migrate/*/context.files`. Your domain: `corpus/migrate/*/GAPS.md`
(new), `docs/corpus.md`, `docs/migrate.md`, `corpus/migrate/LOG.md`.
Do NOT edit any Cixfile, lock, check.sh, or receipt — Cixfile fixes are
queued regeneration work for a later track, not this one.

## Deliverable 1 — a GAPS.md for every case (21)

Per the convention in docs/corpus.md: free-form markdown, header pair

```
Generated: migrate.md@<commit> · <model> · <date>
Status: current            (or: stale — regenerate with <feature/CIP>)
```

then one prose bullet per gap ending in a routing arrow: `→ case`,
`→ prompt`, `→ language (<draft/CIP link>)`, `→ evidence`, `→ refused`,
`→ browser` (page-clarity fixes owned by the browser track), or a new
target if none fits. Provenance is best-effort honest: recover the
generating commit/model from `git log -- corpus/migrate/<case>` and
`.dev/specs/track-corpus*.md` / `corpus/migrate/LOG.md` where possible;
write `unknown` where not. Never invent.

Two language drafts already exist — link them instead of re-reporting:

- `cips/draft/artifact-import.md` — LINK piles, whole-package COPY+PATH,
  the redis no-assembly tension, runtime-path LINK targets (directus's
  mkdir/rm/ln state dance).
- `cips/draft/builder-dev-imports.md` — filestash's CPATH/LIBRARY_PATH/
  PKG_CONFIG_PATH preamble.

Also: every case that is nixpkgs-dissolvable gets a gap line for its
missing twin (`→ case: add Dockerfile-faithful twin` for today's
nixpkgs-only cases; `→ case: add dissolved twin` where only a faithful
conversion exists but nixpkgs packages the app). Per the process section,
both translations will be shown side by side.

Beyond Mathijs's seed feedback below, do your own desk review of each
case against its Dockerfile: dropped ENVs, dropped config, layout
divergence, undeclared parity losses. A gap you find yourself counts
exactly as much as a seeded one.

### Mathijs's seed feedback (2026-08-04, condensed; route each item)

- **adminer**: upstream's ENV version/SHA structure dissolved into the
  FETCH URL (builder ENV can interpolate — prompt fix); COPY to
  filesystem root instead of upstream's /var/www/html; upstream's
  0-upload_large_dumps.ini php settings (upload_max_filesize,
  post_max_size, memory_limit, max_execution_time, max_input_vars)
  dropped with no note — real parity gap.
- **caddy**: nixpkgs-only, uninformative as a translation (twin gap);
  START is `caddy respond` — a probe-shaped toy, not the upstream
  `caddy run --config` contract, yet the table reads ✅; state layout
  (XDG under one STATEDIR) silently diverges from upstream /config +
  /data.
- **directus**: page-status inscrutable ("what is this? blocked on…" —
  the FHS-loader blocker must be stated plainly, → browser/evidence);
  symlink dance + `LINK node /bin/node` → artifact-import draft;
  `COPY ${build}/dist /directus` is the good pattern, say so; a second
  STATEDIR would showcase multiplicity (wallos already shows two —
  cross-reference).
- **dozzle**: red for inscrutable reasons — the two causes (UI build
  non-reproducible; runtime requires Docker's socket, refused) must be
  readable on the page. → browser + evidence.
- **echo-server**: COPY-to-root; LINK-over-IMPORT. Plus the standing
  evidence gap (non-reproducible historical build tree).
- **excalidraw**: Cixfile reads well but the orange is unexplained —
  state the evidence gap plainly.
- **filestash**: LINK slop + COPY to weird dirs; the env-preamble →
  builder-dev-imports draft.
- **mastodon**: only compose.json is visible; `corpus-mastodon-*:checked`
  tags come from per-member Cixfiles + check.sh tagging — provenance must
  be visible. → browser.
- **memcached, nats**: LINK-vs-IMPORT; nixpkgs-only twin gap.
- **nginx**: nixpkgs-only twin gap; unnecessary heredoc.
- **parse-server**: seven enumerated COPYs — either the exclusion is
  deliberate (state what is excluded and why) or copy the deploy unit;
  LOGDIR should mirror upstream's /parse-server/logs (role-dir paths are
  free — prompt fix); `ENV NAME=value` question recorded in
  docs/open-questions.md, cite it.
- **phpmyadmin**: probe-parity vs config-parity — ribbon overclaims to a
  casual reader (upstream config generation untested).
- **redis**: the no-LINK style is the artifact-import draft's open
  question — link it.
- **renovate**: 🔶 is honest but the credentials-unconverted loss must be
  loud on the page.
- **tomcat**: whole-package COPY to root paths + hand-built PATH →
  artifact-import draft; weird virtual fs.
- **verdaccio**: the sed rewrite is unnecessary — STATEDIR can be
  /verdaccio/storage directly (`→ case`, cite migrate.md's own
  role-dir-path rule); upstream's ENVs dropped (parity); "package-manager
  build remains non-green" must become plain language: the build fails
  before producing any item, the Cixfile is untested aspiration.
- **wallos**: heredoc overuse — config pair waits on the FILE…FROM draft
  (cite it), but `include ${pkgs.nginx}/conf/mime.types` inside the
  heredoc is what LINK is for; six-LINK pile → artifact-import draft.
- **watchtower, whoami**: LINK bin + implicit self-import → draft;
  whoami also carries the evidence gap.

## Deliverable 2 — two-axis grading in docs/corpus.md

Rewrite the living-corpus table (and the Ribbons legend) so a casual
reader cannot over-read: split the single ribbon into **Fidelity**
(faithful / declared losses / blocked / refused — one honest clause) and
**Evidence** (desk / build / runtime probe / closed-root). Caddy is the
canary: after your rewrite its row must not look "done". Keep rows
consistent with each case's GAPS.md. Do not re-grade sections 1–3 (wild
compose/k8s) — only the living corpus table and its legend.

## Deliverable 3 — migrate.md addenda (loop 2)

For every `→ prompt` gap, amend docs/migrate.md so a cold regeneration
would not reproduce the slop. Known addenda (write them properly, in the
document's teaching voice, where they belong rather than as an appendix):

1. Mirror the upstream filesystem layout (/app, /var/www/html, …) unless
   there is a stated reason; COPY-to-root is a smell. Role-dir paths may
   and usually should mirror upstream paths.
2. Preserve upstream version/checksum structure as builder ENV binders
   interpolated into FETCH, instead of dissolving versions into URLs.
3. A parity checklist: every upstream ENV, config file, and tuning knob
   is accounted for — translated, dissolved (with the systemd/nix reason),
   or gap-listed in GAPS.md. Silence is the only forbidden disposition.
4. Narrow-vs-unit COPY: enumerating many siblings from one build output
   is a smell — copy the unit, or state what is excluded and why.
5. The GAPS.md contract itself: a conversion is not complete without one
   (point to docs/corpus.md for the convention).

Judgment call on wording is yours — you wrote the current migrate.md
voice. Do not add speculative guidance for unadopted drafts beyond the
existing FILE…FROM mention pattern (cite drafts as pending, never teach
their syntax).

## Deliverable 4 — language-gap report (loop 1)

Collect every `→ language` gap that is NOT covered by the two existing
drafts into a final LOG.md entry titled "Language-gap candidates for the
orchestrator" — one line each: gap, exhibiting cases, why existing
mechanisms don't cover it. Do NOT write CIP drafts yourself.

## Gate

Standard agent tier: `cargo fmt --check`, examples fmt, warning-denied
clippy, full workspace tests, tour regen+drift. You touch no Rust, so
failures you did not cause are reported, not fixed. Do NOT regenerate
the corpus browser. Receipts are synchronous exit statuses in your LOG
with exact repro commands.
