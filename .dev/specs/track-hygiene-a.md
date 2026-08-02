# track/hygiene-a — CIP-90 leg A: clap boundary outside cix-compose

Read AGENTS.md first (focused agent gate; synchronous receipts;
shared-state justification rule). Authoritative:
cips/accepted/0090-test-hygiene.md §3.1 + §5 Decision (clap LITERALLY;
env resolved once at the CLI boundary, arg > env > default; library
code never reads env; global state avoided at all costs) and the §
changelog shared-state amendment. Work in
`/home/mathijs/worktrees/composix/track-hygiene-a` (herdr worktree)
on branch `track/hygiene-a`. Keep `crates/cix-run/LOG.md` current.
PARALLEL FENCE: track/netns runs in cix-compose — cix-compose is
ENTIRELY out of scope this leg (its env reads, ps --json, the tour
filter swap, and the test-Mutex deletions that depend on them are
leg B, after netns). You own cix-run, cix-build, cix-index,
cix-cixfile, cix-common, and the cix CLI crate's non-compose surface.

1. **Boundary config via clap's env feature**: every `CIX_*` env read
   in the owned crates moves to the clap derive layer (`#[arg(long,
   env = "CIX_…")]` on existing/new flags) or, where a value has no
   natural flag, a single boundary resolver in the CLI crate that
   fills a typed config struct passed down. Library code loses ALL
   `std::env::var("CIX_…")` calls. Sweep at least: CIX_STATE_DIR
   (non-compose consumers), CIX_BUILD_WORKSPACE_DIR,
   CIX_PRIVATE_DEVICES_PROBE, the index test vars, watch.rs.
2. **Shared-state targets** (CIP-90 changelog): unify the duplicated
   `INTERRUPTED` signal flags (watch.rs + runtime.rs) into one
   justified cix-common atomic with the justification comment; make
   `NONCE_COUNTER` an injected generator (config-struct member with a
   default), deleting the static.
3. **Tests construct config directly** — every `set_var`/`remove_var`
   in the owned crates' tests disappears (proj1's
   WORKSPACE_DIRECTORY Mutex dies with it; fmt/lock_nix/index tests
   likewise). No mutex-guarded env anywhere.
4. **The lint**: a gate check (extend scripts/check-source-size.sh
   style — own small script) denying `std::env::var("CIX_` and
   `set_var` outside the declared boundary modules; cix-compose
   temporarily allowlisted WITH a leg-B pointer comment.
5. Docs: a short "configuration" section in docs/cixfile.md or the
   CLI docs naming the precedence rule (arg > env > default).

Gate (agent side): fmt / examples fmt / warning-denied clippy / full
workspace tests / tour regen + drift / focused: vm-dogfood (flag
plumbing touches runtime). Full matrix at the orchestrator gate.
Commit on this branch when green.
