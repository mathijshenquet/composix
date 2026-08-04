Generated: migrate.md@0f63d00 · unknown · 2026-08-02
Status: stale — regenerate with CIP-81

- The scheduled APP runs only `renovate --version`; it never performs a repository renovation, so the green timer/log receipt is a mechanism probe rather than an application conversion. → case
- Repository configuration and tokens are entirely absent even though they are required for useful execution; make that loss prominent wherever the case is summarized. → browser
- CIP-81 now provides credential files, but the case has not converted the upstream config/secret delivery or demonstrated Renovate consuming it. → case
- The upstream CronJob's schedule options, concurrency/deadline/retry policy, cache, command hooks, and pod placement knobs are reduced to hard-coded `daily` plus persistence without itemized dispositions. → case
- This nixpkgs-only mechanism demo has no upstream-faithful CronJob twin. → case: add upstream-faithful twin
- The closed-root receipt proves timer activation, a version command, and indexed logs only; it is not evidence for authenticated repository work. → evidence
