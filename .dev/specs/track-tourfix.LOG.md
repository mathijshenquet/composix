# track/tourfix work log

## 2026-07-29T13:16:43Z — correction round 1 start

- Read the correction spec, current generated tour, D13/D19/D36, and the runner's foreground path. The old-systemd `Unknown assignment: PrivatePIDs=yes` line leaks only from `cix debug`: `run_transient_foreground` streams `systemd-run` stderr verbatim while also collecting it for the later fallback diagnostic.
- Plan: make the harness drop the complete `Unknown assignment: <property>` line class; keep that line captured in the runner's diagnostic result but suppress its direct stream copy, so the existing loud cix fallback warning owns the user-visible explanation. Add unit/regression coverage, regenerate the tour, then run the requested full gate.

## 2026-07-29T13:18:00Z — correction implemented

- `run_transient_foreground` now reads stderr one line at a time. Every line remains in `ForegroundResult.stderr`, which `failed_attempt` folds into `with_unit_diagnostics`; only the exact old-systemd `Unknown assignment: <nonempty property>` class is withheld from direct stderr streaming. Thus cix retains the raw fact for fallback classification/context, but the transcript exposes only its stable, loud warning pair.
- Added runner coverage for LF and CRLF forms plus nonmatching diagnostics. The harness now removes the complete bare old-systemd line class before rendering, with a regression input for `PrivatePIDs=yes`; it consumes the property to end-of-line, so no supported/unsupported property spelling can vary the page.
- `cargo fmt --all` and both focused regressions passed. Removed an unused import found by that run. Next: regenerate and verify the complete tour, then run the workspace gate.

## 2026-07-29T13:19:00Z — gate correction

- Tour regeneration, deterministic rendering, and committed-document drift all pass. The generated pages do not change on this systemd ≥257 host; the regression proves the old-host-only line is swallowed.
- Initial workspace clippy run rejected a nonminimal boolean in the new diagnostic predicate. Applied its equivalent `is_none_or` form; rerunning the full gate now.

## 2026-07-29T13:21:00Z — correction gate passed

- Full gate passed: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (including live user-run integration plus tour determinism and committed-doc drift). The ignored generator was also run explicitly before the drift check.
- Swept all generated tour transcripts for raw systemd output. They contain no bare systemd diagnostics: `cix run` reports only cix-owned D13 warnings, `cix ps` renders parsed JSON rather than `systemctl` prose, and the sole `cix debug` transcript goes through the foreground stderr path corrected here. The only other historical systemd variance, the optional `[systemd-run] ` description prefix, is already normalized by class. Therefore old/new systemd can vary assignment lines, description prefixes, and fallback details without affecting a rendered page; nonempty cix-owned semantic warnings remain intentionally visible.
- No generated page changed on this host; that is expected because its systemd accepts `PrivatePIDs`. Next: final diff review and commit on `track/tourfix`.

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
