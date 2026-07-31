# track/nixcompare — gitsitter three ways: upstream flake vs crane vs Cixfile

Read AGENTS.md first. Context: design.md D67 (strata; store-native vs
derivation-native — the doc must state that distinction frontally, never
oversell "pure nix"), D68 (ITEM), D62/D65 (naming/refs). This is a
positioning-with-receipts round: every claim measured, every number dated.
Scope: docs/nix-build.md (new) + examples/compare/gitsitter/** + tour-style
receipts inside the doc. NO crate changes.

## Subject
github:mathijshenquet/gitsitter (Rust daemon, public). Three build routes:
1. **Upstream flake**: `nix build github:mathijshenquet/gitsitter` (inspect
   what its flake actually uses; report its definition honestly).
2. **crane**: author a minimal crane flake in examples/compare/gitsitter/crane/
   (standard buildPackage with dep-artifact splitting; keep it idiomatic, no
   golf).
3. **Cixfile**: examples/compare/gitsitter/cix/ —
   `FROM github:mathijshenquet/gitsitter AS src` (remote source binding is
   itself a demo), BUILDER with IMPORT/FETCH cargo/RUN offline build, ITEM
   output holding the binary tree (D68 — no manifest; this is stratum 2
   pure).

## Measurements (all with exact commands + host note + date in the doc)
- Authoring: LOC of each build definition + a concept inventory (what must
  the author understand for each route).
- Cold build wall-clock (clean store state for the subject where feasible;
  document exactly what "cold" meant per route — nix builds: no substitutes
  for the crate deps; cix: `--cold`).
- No-op rebuild; warm rebuild after a one-line source change (use a pinned
  patch so it is reproducible).
- Closure size (`nix path-info -sSh`) of each result.
- Determinism: build each route twice; report path/byte equality per route
  honestly (cix route: warm-vs-`--cold` equality too — the D47e bridge
  check in action).
## Distribution chapter (the stratum-3 demo)
Tag the ITEM (`--namespace` not needed: single member), `cix serve
--with-store`, pull+run-nothing (it is an item) on a second store location or
document the pull flow against the local serve honestly. Frame: "I want to
distribute gitsitter — what now?" answered with commands.

## Doc shape
docs/nix-build.md, tour-style: prose → command → real output → observation.
Honest comparisons BOTH ways (expected: crane wins hermetic purity +
flake-interop; Cixfile wins authoring weight + warm loop; upstream flake =
baseline). The store-native vs derivation-native trade (D67b) gets its own
early section. End with the honest-fit guidance: when to pick which.

## Gate
All three builds green and re-runnable (record exact repro); doc samples =
the actual committed example dirs (no drift between doc text and files);
`devenv shell -- cargo test --workspace` untouched-green smoke; diff confined
to docs/nix-build.md + examples/compare/**. LOG: crates/cix-cixfile/LOG.md.
Commit on this branch when green.
