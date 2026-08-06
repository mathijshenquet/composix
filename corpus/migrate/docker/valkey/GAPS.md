Generated: migrate.md@current · independently rechecked · 2026-08-06
Status: current

- The faithful 8.1.9 build, `valkey-cli PING` probe, and empty-workspace cold replay pass. CIP-87 tracing now classifies exclusive file creation and reads beneath same-step-created directories as outputs rather than incoming observations, including across PID-namespace child processes. → evidence
- Docker's fixed uid/gid, root-time `chown`, and `/data` ownership model dissolve into DynamicUser and the declared state role; this host's service stop can still encounter a read-only `/data` during Valkey's final RDB write. → case
- The dissolved twin deliberately follows nixpkgs' Valkey package rather than the faithful 8.1.9 source build and its Docker patches. → evidence
