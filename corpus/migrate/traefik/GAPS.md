Generated: migrate.md@a5d6a3b · terra · 2026-07-30
Status: current

- The conversion starts a synthetic ping endpoint on ports 8081/8080 instead of the upstream generic `traefik` contract on port 80. → case
- The package binary is exposed through an implicit `/bin` link instead of a declared artifact tool import. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- The ping receipt does not compare the selected package version with upstream 3.5.6 or exercise reverse-proxy configuration. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
