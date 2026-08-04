Generated: migrate.md@a5d6a3b · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- The upstream `nats-server.conf` is dropped and replaced by `-m 8222`; the cluster listener on 6222 and the upstream config contract are therefore absent. → case
- The package binary is exposed through an implicit `/bin` link instead of a declared artifact tool import. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The monitoring health probe does not establish selected-package version parity with upstream 2.12.14 or exercise a client message round-trip. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
