# track/tourfix — normalize host-specific degraded-fallback warnings in the tour

CI failed on main: docs/tour/09-running-listener.md drifts between hosts because WHICH
sandbox properties a user manager rejects is host-specific (dev host: mount-ns rejected →
"retrying without PrivateUsers, PrivatePIDs, …"; CI runner: capability controls rejected →
"retrying after dropping AmbientCapabilities, …"). The committed page encodes the generating
host's fallback path. See CI run 30452724731.

Fix in the tour harness normalizers (crates/cix/tests/tour.rs): collapse the degraded
fallback warning lines — the "user manager rejected …" line and the "retrying …" line — into
stable placeholders that keep the semantic fact (a loud degraded fallback happened) but
swallow the host-variable property list. Regenerate the tour; drift + determinism tests
green. The normalization must be host-independent BY CONSTRUCTION (placeholder swallows the
variable part) — state in the LOG why no reachable host output can differ post-normalization.
Gate: cargo test workspace green, fmt/clippy clean, commit on track/tourfix.
Keep .dev/specs/track-tourfix.LOG.md current.

## Correction round 1 (CI run 30453603963)

A second host-variance class: on systemd < 257 hosts, `systemd-run` emits
`Unknown assignment: PrivatePIDs=yes` as a bare stderr line (newer systemd knows the
property and only rejects it at runtime). It appears in 11-debugging-service.md on CI only.
Fix BOTH levels: (1) harness normalizer drops/normalizes `Unknown assignment: <property>`
lines; (2) product question — cix's probe/fallback path deliberately summarizes systemd
diagnostics in its own loud warnings, so raw systemd chatter about a property we are about
to drop should be captured into the (already normalized) diagnostics blob rather than
leaking verbatim; implement that capture in the runner if it is a contained change, else
record why not in the LOG. Also sweep the OTHER tour pages for any remaining raw
systemd-emitted line classes and normalize the class, not the instance — reason about what
old/new systemd can emit differently, state the argument in the LOG. Gate: workspace tests,
regenerated tour, fmt/clippy, commit on track/tourfix.
