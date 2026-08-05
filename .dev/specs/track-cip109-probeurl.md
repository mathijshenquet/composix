# track/cip109-probeurl — implement CIP-109 (URL probe targets)

Read first: `cips/accepted/0109-probe-url.md` (decision). Implement
exactly: URL form canonical (`READINESS http://127.0.0.1/health.php IN
20s`, `tcp://host:port`, bare `notify`; scheme = probe kind, standard
URL port defaulting), path-only sugar when the service declares
exactly one PORT, teaching diagnostics both directions, old two-token
form rejected with a rewrite hint. Sweep all corpus/examples/tour
probe lines to the URL form; regenerate tour/browser. Also fold in
the adjacent small harness fix (two exhibits: wallos, mailpit): probe
execution must not invoke a workspace-local `target/debug/cix` under
ProtectHome — resolve the runtime helper from the store/installed
path; add a regression assertion.

Discipline: branch `track/cip109-probeurl`, LOG `crates/cix-run/LOG.md`;
full agent gate tier (contract-keyed selector prices it), value-checked
capture receipts, bounded VM parallelism. Merge semantically if main
moves. Clean branch; do not merge.
