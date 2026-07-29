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
