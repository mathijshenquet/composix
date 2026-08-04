# track/tourpolish — file presentation and chapter fixture polish

Source of truth: `crates/cix/tests/tour.rs`. Generated pages live in
`docs/tour/` and must not be edited directly.

## Scope

1. Replace every displayed `cat` transcript with a harness `show_file`
   affordance. Each file gets its own relative-path label and fenced block.
   Language mapping: Cixfile → dockerfile, `.nix` → nix, `.conf` → nginx,
   `.py` → python, `.json` → json, `.html` → html, otherwise plain.
   Render from the actual file and preserve assertions against its raw content.
2. Build Chapter 6's listener fixture from a canonical Cixfile with `LISTENER`;
   keep its checked-in Python `LISTEN_FDS` probe as the copied source.
3. Keep Chapter 2's `STATEDIR /opt/nginx/state` CIP-91 materialization example
   and explain that deliberate choice in one sentence.
4. Rename Chapter 2's demo service from the abstract `language` name to a name
   appropriate for its demo site.

## Gate

Standard agent tier: workspace formatting, canonical example formatting,
warning-denied all-target clippy, full serialized workspace tests, tour
regeneration plus zero drift, and three consecutive synchronous tour
determinism runs. No VM scenario changes are expected for this tour-only track.
