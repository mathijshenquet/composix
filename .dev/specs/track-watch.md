# track/watch — CIP-76: `cix watch`

Read AGENTS.md first. Authoritative: docs/cips/0076-devloop.md
(§3 + §5 Decision). Work in `.worktrees/watch` on branch `track/watch`.
Keep `crates/cix-cixfile/LOG.md` current.

1. New subcommand `cix watch [PATH]` (default `.`): watch the Cixfile
   context for changes → debounce (~300ms, coalescing bursts) → warm
   rebuild → restart exactly the changed services → loop. Ships NOISY
   (build output streams as-is; no quiet mode — CIP-76 decision).
2. Two modes by what PATH contains:
   - compose.json present: rebuild changed members, then the
     restart-changed path (`cix up` semantics) — compose-wide watch.
   - bare Cixfile: `cix build` on change (the warm-loop feedback alone);
     print the resulting item path per round.
3. Context-derived ignores — the load-bearing part (a wrong ignore set =
   infinite rebuild loop): ignore `.git`, the builder workspace location
   (CIX_BUILD_WORKSPACE_DIR and its default), `Cixfile.lock` writes cix
   itself performs, target/, and .gitignore'd paths (reuse the `ignore`
   crate wiring from cix fmt). Prove the no-self-trigger property with a
   test: a watch round that rebuilds (thus writing lock/workspace) must
   not schedule a second round.
4. Use the `notify` crate (new dep) for FS events; fall back to polling
   only if notify init fails, with a stderr note.
5. Ctrl-C exits cleanly (no orphaned build). Exit code 0 on interrupt.
6. Docs: docs/cixfile.md workshop section — the dev-loop story incl.
   the split: framework hot reload belongs in `nix develop`, cix watch
   is the artifact loop (CIP-76 wording).
7. Tests: CLI-level — a scripted edit triggers exactly one rebuild
   (tempdir fixture, short debounce override via hidden env var);
   ignore-set unit tests; no-self-trigger test from (3).

Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit on
this branch when green.
