# track/tourfix work log

## 2026-07-29T00:00:00Z — start

- Read `AGENTS.md`, the repository journal, D19 in `docs/design.md`, and the tour-fix spec.
- Scope: make `crates/cix/tests/tour.rs` swallow every host-variable rejected-property and retry-property list as stable degraded-fallback placeholders, regenerate `docs/tour/`, then run the full Rust gate and commit on `track/tourfix`.
- Rationale to verify: both normalizers will match the complete diagnostic lines up to their newline, so no reachable variation in systemd's error text or fallback property list can survive into rendered tour output.

## 2026-07-29T00:10:00Z — normalization and regeneration

- Replaced the two partial diagnostic normalizers with one multiline, paired matcher: from either `user manager rejected …` spelling through the corresponding `retrying …` line. It renders two fixed warnings that explicitly state the rejected controls required a D13 degraded fallback.
- Added a regression test covering both capability and mount-namespace rejection paths, including a multiline host error. The matcher consumes all text before the retry line and the retry line's entire property list; therefore every reachable variation in those host outputs normalizes identically.
- Ran `cargo fmt --all`, the focused regression test, and `cargo test --package cix --test tour -- --ignored generate_tour`; all passed and the tour was regenerated. Next: separate deterministic/drift checks, inspect the diff, then full workspace gate.

## 2026-07-29T00:20:00Z — gate passed

- Tour verification passed: `cargo test --package cix --test tour` (normalization regression, deterministic rendering, foreign-unit isolation, and committed-doc drift check).
- Full requested gate passed: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
- Final change set is limited to the tour normalizer/regression test, regenerated affected pages (07, 09, and 11), and this append-only track log. Next: final diff review and commit on `track/tourfix`.

## 2026-07-29T00:25:00Z — complete

- Committed the gated change on `track/tourfix` as `fix: normalize tour degraded fallbacks`.
- Worktree is clean. No open items for this track.
