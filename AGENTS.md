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
- Work happens on `track/<name>` branches in herdr-managed worktrees (`herdr worktree create --branch track/<name> --label <name> --no-focus` — worktree + attachable workspace in one, so the fleet is visible in Mathijs's UI; Mathijs 2026-08-02). Pre-existing `.worktrees/<name>` checkouts remain valid until their track lands. Spec file per track in `.dev/specs/`.
- New feature strata get new modules; keep crate module maps current.
- Keep your assigned LOG.md current (append-only, timestamped) — it is the durable memory.
- "Green" claims by agents get independently re-verified before merge — design for that: leave exact repro commands in your LOG.
- A receipt is a SYNCHRONOUS exit status you observed, never detached or quiet output (lesson of 2026-08-02: three consecutive false greens on one scenario from reading detached-build output as success).
- Shared ownership and interior mutability (Arc/Rc/Mutex/RwLock/RefCell/atomics/statics) require a stated justification comment at the site; unjustified uses are review-rejected. "Didn't feel like Rust semantics" is not a justification — it is a complexity hazard (Mathijs, 2026-08-02). Canonical acceptable uses: signal-handler atomics, process-wide instrumentation counters.
- Decisions live in docs/design.md only; propose amendments there, don't fork design prose into other files.
- Track gates (agent-side): fmt / examples fmt / warning-denied clippy / full workspace tests / tour regen+drift, plus the FOCUSED VM scenarios your track touches or adds. The inner-loop VM command is `devenv shell -- nix run .#progressive-vm-check`: it derives selection from the current and base scenario derivations, prints every selected and skipped scenario, and accepts `--full` for the complete matrix. The full `devenv shell -- nix flake check -L` matrix runs once per track, at the orchestrator's independent pre-merge gate — that layer is load-bearing and never skipped (amended 2026-08-02 for gate speed; supersedes the full-matrix-per-agent rule of 2026-07-31 — the guarantee moved layers, it did not weaken: agents never hand-pick what COUNTS as green, they run a smaller declared tier).
- Ledgers stay current (Mathijs, 2026-08-02): a track that lands or changes behavior re-grades the affected docs/docker.md and docs/corpus.md rows in the same track. Desk grades vs verified receipts stay honestly distinguished. Extension (2026-08-04): the same track greps `corpus/migrate/{docker,k8s}/*/GAPS.md` for its CIP/draft name and flips exhibiting cases to `Status: stale — regenerate with <feature>` (see docs/corpus.md "How this corpus is maintained").

## Session close (orchestrator)
Append a dated entry to `.dev/LOG.md`: merged work, decisions taken (with D-numbers), open items *with Mathijs* vs open items *for agents*.
