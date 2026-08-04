Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- Direct interpolation of the nixpkgs executable leaves an empty-looking artifact; CIP-91 resolves the canon as artifact IMPORT plus a bare command. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The PING receipt does not establish upstream 7.4.8 parity, its compile-time protected-mode/TLS choices, or entrypoint argument behavior; Docker mode was not rerun. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
