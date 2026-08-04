Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- Docker and nginx serve port 80, and the Cixfile faithfully declares 80; `check.sh cix` instead probes 18090 without passing a Cix port override, so its red result is an acceptance-harness defect rather than evidence that the service chose the wrong port. → evidence
- The helper normalizes the generated sitemap timestamp and drops the non-runtime `sw.js.map` debug map to stabilize otherwise volatile output; that deliberate source-build deviation remains visible. → case
- Docker's `TARGETARCH` dependency selection is replaced by the locked local Cix host package universe, so cross-architecture behavior is not reproduced. → case
