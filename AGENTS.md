# composix — agent context

Nix-native docker analogue in Rust. Four parts: index (tag DB / serve / pull),
spec+run (systemd unit generation, hardening), compose (multi-service stacks),
Cixfile (Dockerfile-ish adoption bridge with .nix escape hatch).

## Where truth lives (read in this order to rebuild context)
1. `.dev/LOG.md` — session journal, newest entry first. Top entry = current state + open items.
2. `docs/design.md` — decision registry (D-numbers, cite as "D31"). "Building now" = active scope; "Decisions so far" is authoritative over any other doc on conflict.
3. Per-crate `crates/*/LOG.md` — track-level detail, only when touching that crate.

Other landmarks: `.dev/specs/track-*.md` (past agent specs; write new ones here),
`docs/tour/` + tour harness in tests (executable docs, drift-checked),
`examples/` (each is e2e-verified; don't break them silently), `docs/docker.md`
(honest gap ledger vs docker).

## Environment
- devenv + direnv (`.envrc`); cargo workspace, binary is `cix`.
- Tour/VM tests are the real gate: `cargo test` + the NixOS VM check via the flake.

## Conventions
- Work happens on `track/<name>` branches in `.worktrees/<name>`; spec file per track in `.dev/specs/`.
- Keep your assigned LOG.md current (append-only, timestamped) — it is the durable memory.
- "Green" claims by agents get independently re-verified before merge — design for that: leave exact repro commands in your LOG.
- Decisions live in docs/design.md only; propose amendments there, don't fork design prose into other files.
- Track gates END with the full `devenv shell -- nix flake check -L` — never a cherry-picked VM subset. (Lesson of 2026-07-31: thirteen merges passed a red CI because per-track gates ran hand-picked checks.)

## Session close (orchestrator)
Append a dated entry to `.dev/LOG.md`: merged work, decisions taken (with D-numbers), open items *with Mathijs* vs open items *for agents*.
