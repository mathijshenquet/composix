# parse-server migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

- `./check.sh cix` — synchronous exit 0; the Mongo-backed `/parse/health` receipt passed.
- `target/debug/cix build --cold .#parse-server` — synchronous exit 0; the pinned npm snapshot replayed, Babel compiled 197 files, and produced `/nix/store/nnsfywf2zk5mvz5sxcl0lrvsap48zy56-cix-item-parse-server`.

This refutes the staging cold-divergence claim for the regenerated Cixfile.
