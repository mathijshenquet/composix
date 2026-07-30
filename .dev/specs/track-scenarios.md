# track/scenarios — the scenario tier: escalating VM scenarios + a TDD frontier

Read AGENTS.md first. Context: docs/compose-tree.md (the ladder + tree model),
docs/design.md D42–D45, the existing `nix/vm-dogfood.nix` check (your structural
template). This track builds the *test* tier that the next feature waves must be driven
by — tests first, features later. Nothing in this track adds CLI verbs or changes
runtime semantics.

## Placement & fencing

- New scenarios live in `nix/scenarios/<name>.nix`, wired as
  `checks.${system}.scenario-<name>` in `flake.nix` (small wiring edit only).
  `vm-dogfood.nix` stays untouched. `nix flake check` picks the tier up wherever CI
  already runs it; verify that and note the CI reality in your LOG (adjust the workflow
  ONLY if scenarios would otherwise not run in CI at all).
- Do NOT touch `crates/cix-cixfile`, `examples/build/**` (track/items owns those,
  running concurrently), and make no changes to `crates/*` source except the one test
  file named below. Scenarios drive the real `cix` binary inside VMs, using
  `examples/pack/*`, `examples/compose/stack`, and inline manifests in the
  vm-dogfood style (cixManifest 2 inline manifests are fine — the runner accepts 1–4).
- Each scenario must stay under ~5 minutes of VM time; prefer fewer, denser scenarios
  over many boots.

## The escalating scenarios (the ladder, made executable)

1. **scenario-lifecycle** — one composite over time: `cix up` the stack →
   HTTP round-trip through the edges → mutate the compose file (env override change) →
   `cix up` again → assert *restart-changed selectivity*: only the changed service's
   unit restarted (compare `ActiveEnterTimestamp` of the untouched services before/
   after), generation count incremented → `cix rollback` → assert previous generation
   active, env override reverted, state dir contents SURVIVED both transitions (write a
   sentinel file into a state dir via the service before the update) → `cix down` →
   assert no cix units remain.
2. **scenario-side-by-side** — two instances on one host, honestly: bring up the same
   stack twice under different composite names. Assert what MUST hold today: units,
   slices, state dirs, edge runtime dirs are fully disjoint (path-namespaced); both
   stacks serve concurrently on their two distinct host binds. Then assert the honest
   wart: a third up with a *conflicting* host bind (same address:port as instance one)
   fails LOUDLY (capture the failure mode precisely — this assertion documents current
   truth). Mark the future with a clearly labelled disabled block:
   `# D43 FRONTIER (flip when pod-ness lands): identical internal ports without any
   bind conflict once both claim network: pod` — commented out or gated off, with the
   D-number, so the next wave flips exactly this.
3. **scenario-update-repin** — the D44 flow at v0 scale: publish v2 of one item into
   the VM-local index, `cix up` with the service on `update: track` → assert only that
   service restarted and its store path moved; the pinned sibling did not; generation
   incremented. Then `cix rollback` → assert the OLD store path is active again
   (generations pin content, not tags). Leave a labelled
   `# D44 FRONTIER: --update <edge> selective repin on nested composites` marker.
4. **scenario-gc-survival** — pointers vs bytes: after the ups/republishes above, run
   `nix-collect-garbage` in the VM. Assert: the active generation's paths survive
   (profile roots), the index's CURRENT table item + all referenced store paths survive
   (D45 roots), and `cix ls`/resolve still work. Assert honestly: a superseded
   (historical) table item is NOT rooted — if GC reclaimed it, `cix index history`
   walks the shortened chain without crashing.
5. **scenario-observability** — what exists today, proven: `journalctl -u
   cix-<comp>-<svc>` returns that service's lines (and not its sibling's);
   `systemctl status` on the slice shows the member units; cgroup accounting is visible
   (`systemd-cgtop -1` or reading `/sys/fs/cgroup/.../cix-<comp>.slice`). Close with
   comments citing the ledger rows that remain open (no `cix logs`/`ps`/`stats` — do
   not build them here).

## "Goed testen" — the index concurrency hammer

One new test file: `crates/cix-index/tests/hammer.rs`, `#[ignore]`d (slow), run
explicitly in this track's gate and wired into scenario-gc-survival's VM if cheap:
N OS processes (spawn the test binary or a tiny helper via `std::process`) performing
interleaved `publish_many`/`yank` bursts against ONE store for M rounds. Afterwards
assert: the final table is exactly the linearization implied by the per-process success
counts (no lost updates — every successful flip's tags are present or provably
yanked-later), `parent` chain from current pointer walks without gaps as far as items
exist, and every CAS conflict surfaced as the distinct retry/`PointerChanged` path,
never as corruption. Keep runtime < 60s.

## Gate (leave exact repro commands in your LOG)

- Each `nix build .#checks.x86_64-linux.scenario-<name>` green, locally.
- `cargo test -p cix-index --test hammer -- --ignored` green.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` untouched-green; tour untouched (zero diff — this track
  builds scenarios, not tours).
- Your LOG (`nix/LOG.md`) records per scenario: what it asserts as *current truth*,
  which labelled FRONTIER markers exist (D-number + one line), and the exact flip
  condition for each. That list is the TDD contract for the next waves.
