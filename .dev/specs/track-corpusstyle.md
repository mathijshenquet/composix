# track/corpusstyle — Cixfile authoring canon + corpus browser v2

Read AGENTS.md first (amended gate convention; synchronous receipts).
Work in `/home/mathijs/worktrees/composix/track-corpusstyle` (herdr
worktree) on branch `track/corpusstyle`. Keep
`crates/cix-cixfile/LOG.md` current. FENCE: track/closedroot
(crates/cix-run + a new audit scenario) and track/vmslim
(nix/scenarios/lib.nix) run concurrently — touch neither area. Your
domain: corpus Cixfiles + docs + the corpus browser generator
(crates/cix/tests/corpus.rs + docs/corpus/).

Mathijs's review of the wallos page (verbatim intent): heredocs keep
appearing where real files belong; the build RUN is an unreadable
&&-chain; the browser needs syntax highlighting, a smaller font, and
a better design.

1. **Authoring canon** (record in docs/migrate.md — the teaching
   prompt agents follow — and a style note in docs/cixfile.md):
   - File content lives in real files next to the Cixfile and enters
     via COPY. `FILE <<heredoc` only when the content needs `${…}`
     interpolation, and only until the FILE…FROM draft
     (cips/draft/file-from.md) is adopted — reference it as the
     intended dissolve, do NOT implement it (unadopted).
   - No && chains beyond two commands: multiple RUN steps (read-set
     keying makes them cache well) or a heredoc RUN with one command
     per line.
2. **Apply the canon to the corpus**: rewrite
   corpus/migrate/wallos/Cixfile as the exemplar (setup/start scripts
   become real files + COPY — they interpolate nothing; the giga-RUN
   becomes readable steps; the two configs stay heredoc WITH a
   canon-comment naming why: interpolation). Sweep the other corpus
   Cixfiles for the same two smells and fix where mechanical. Every
   touched case's `./check.sh cix` must re-pass — synchronous
   receipts per case in the LOG.
3. **Corpus browser v2** (generator-side, static output only — Pages
   serves plain HTML, no JS dependencies):
   - Syntax highlighting at generation time as inline spans:
     Dockerfile, Cixfile (write a small lexer: directives, `${…}`
     interpolations, comments, heredoc bodies), nix, JSON, YAML.
     Keep the palette restrained and readable on light+dark.
   - Typography: monospace code at a comfortable reading size
     (~13px-equivalent, not the current oversized rendering);
     tighter page chrome; the two columns must stay usable on a
     laptop width with per-column horizontal scroll.
   - Keep pages fully self-contained (inline CSS) and deterministic;
     the drift test stays the proof.
4. Regenerate all corpus pages; drift test green.

Gate (amended convention): fmt / examples fmt / clippy / workspace
tests / tour regen + drift / focused: the corpus generator + drift
test and every touched corpus check.sh. Commit on this branch when
green.
