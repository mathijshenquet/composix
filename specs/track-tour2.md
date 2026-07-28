# Track: tour2 — extend the literate tour with serve + pull

Read `DESIGN.md` (esp. D12, D17 v2, D18 v2, "HTTP surface", "The org workflow") and the
existing harness `crates/cix/tests/tour.rs` + `docs/tour.md` — you are EXTENDING that harness
with new scenarios in its established style; follow its conventions (Doc builder, `sh` helper,
normalization, drift check, determinism test). Where ambiguous, choose the boring option and
note it in your LOG — do NOT expand scope.

## Ground rules

- Work log: append to `crates/cix/LOG.md`.
- Territory: `crates/cix/tests/`, `docs/tour.md` (via the generator ONLY — never hand-edit),
  and dev-deps in `crates/cix/Cargo.toml` if needed. Nothing else.
- Commit as you go. Done gate: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace` green (drift + determinism included); generated doc reads well.

## New scenarios (appended after the existing three, exactly these)

4. **Serving your store.** Narrative: publication is not a ceremony — serving exposes your bare
   tags at whatever URL reaches the box. Reuse a tagged fixture; start `cix serve --with-store
   --listen 127.0.0.1:<port>` as a child process (pick a fixed port constant; normalize it in
   doc output to `8420`); `curl` the JSON representation of `/{name}:{tag}` with the
   `Accept: application/vnd.cix+json;version=1` header and show it; state in prose that the
   same URL in a browser is an informative HTML page (do not dump the HTML into the doc — show
   `curl -s … | head -c 120` or similar as a teaser only). Stop the server at scenario end.
2. Wait — numbering: keep markdown headings unnumbered like the existing doc; the list here is
   for you.
5. **Pulling on another machine.** Narrative: a second machine is just a second state dir.
   With a fresh `CIX_STATE_DIR` (call the prompt `consumer $` vs `publisher $`, gitsitter
   style): `cix pull 127.0.0.1:<port>/my-app:v1 --as my-app` — show the entry resolve and
   `cix ls -l` with the upstream recorded. Prose: the qualified ref is self-describing; `--as`
   adopts it under a bare local name; the mirror/adoption distinction.
6. **Tags move; pull follows.** On the publisher, retag `my-app:v1` to a second store path
   (or tag `my-app:v2` and retag `latest`— choose the simplest that matches existing scenario 2's
   fixtures). On the consumer, bare `cix pull` (no args) — show it refreshing the moved tag.
   Prose: mutable names over immutable paths, refreshed like git remotes, GC follows the pins.

## Constraints

- The serve child process must be reliably cleaned up (kill on drop) even when assertions fail.
- Wait for serve readiness by polling the HTTP endpoint, not by sleeping a fixed duration.
- New normalizations you will need: the fixed port, any store-path hashes in JSON, createdAt
  timestamps (the existing normalizer may already cover these — reuse it).
- Scenarios must remain independent: each builds its own state dirs; no cross-scenario state.

## Done criteria

Green gate; regenerated `docs/tour.md` committed; LOG.md final summary with deviations and
open questions.
