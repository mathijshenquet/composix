# track/gcroots — D63(b): unit-lifetime GC roots for cix run

Read AGENTS.md first. Authoritative: design.md **D63(b)**. Scope: crates/cix-run
(+cix-common if the root helper fits better there), tour only if transcripts
drift. Runs parallel to track/corpuspolish (corpus-only) — do not touch corpus/.

1. On `cix run` (system mode): before/at unit start, register an indirect nix
   GC root for the item store path: symlink `/run/cix/gcroots/<unit>.root →
   <store item>` plus the `/nix/var/nix/gcroots/auto` registration
   (`nix-store --add-root <link> --indirect -r`-equivalent; use the existing
   cix-index root plumbing if reusable — it already registers indirect roots
   for tags). Create `/run/cix/gcroots` as needed (0755 root).
2. Inject `ExecStopPost=` removing the /run symlink (D48e: a visible unit
   property, no daemon). A dangling auto-side link is pruned by nix itself —
   verify that claim against the local nix and record the observation in the
   LOG. The unit must still stop cleanly if the link is already gone.
3. User mode (`--user`, degraded): same mechanism under `$XDG_RUNTIME_DIR/cix/
   gcroots/`; if indirect-root registration needs privileges the user lacks,
   degrade LOUDLY in the existing D36 banner style (state that the run is not
   GC-protected) rather than failing the run.
4. Tests: unit generation includes the ExecStopPost and the root path; an
   integration test proving the root exists while a unit runs and is gone
   after stop (system-mode test in the style of the existing
   system_projection tests); `cix ps`/stop flows unaffected.
5. Docs: docs/docker.md GC/prune rows if they mention lifecycle; a sentence in
   docs/cixfile.md's run section that anonymous runs are GC-protected for the
   unit lifetime.

Gate: `devenv shell -- cargo fmt --all --check`; warning-denied workspace
all-target clippy; `cargo test --workspace`; tour regen + drift + determinism
twice (expected no-op — say so if so); `devenv shell -- nix build
.#checks.x86_64-linux.vm-dogfood --no-link -L`. Exact repros + unit cleanup in
crates/cix-run/LOG.md. Commit on this branch when green.
