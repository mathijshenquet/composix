Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: current

- The package-only conversion replaces upstream's normal nginx contract and entrypoint-driven template/resolver/worker tuning with a synthetic one-server config that returns a fixed string. → case
- The non-interpolating nginx configuration is an unnecessary heredoc instead of a normal checked-in file. → case
- The receipt reports nixpkgs nginx 1.30.4 while the Dockerfile pins 1.31.3, and Docker mode was not rerun. → evidence
- Upstream requests graceful `STOPSIGNAL SIGQUIT`; the manifest has no stop-signal declaration and silently uses systemd's default. → language ([recorded stop-signal gap](../../../docs/open-questions.md#proposed-one-line-dispositions-awaiting-mathijs-batch-blessable))
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
