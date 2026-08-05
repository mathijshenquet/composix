Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- The faithful 8.1.9 build and `valkey-cli PING` probe pass, but cold replay exits 1 after compiling at `libbacktrace/.libs/stVX6SFe`: its recorded warm read is `Some(Absent)` and the cold trace records `None`. → language (CIP-87)
- Docker's fixed uid/gid, root-time `chown`, and `/data` ownership model dissolve into DynamicUser and the declared state role; this host's service stop can still encounter a read-only `/data` during Valkey's final RDB write. → case
- The dissolved twin deliberately follows nixpkgs' Valkey package rather than the faithful 8.1.9 source build and its Docker patches. → evidence
