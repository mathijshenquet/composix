Generated: migrate.md@c43ae9b · terra · 2026-07-30
Status: current

- The package binary is exposed through an implicit `/bin` link instead of a declared artifact tool import. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- The receipt exposes concrete version skew—upstream 1.6.45 versus nixpkgs 1.6.42—and does not establish parity for the upstream SASL, TLS, extstore, or proxy build features. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
