Generated: CIP-102 volatile-fetch sweep · 2026-08-05
Status: current

- The Dockerfile's copied `/entrypoint.sh` is absent from the supplied context, so its arbitrary-argument behavior cannot be audited. The service starts `traefik --ping=true` directly and the receipt proves only that ping endpoint, not a reverse-proxy configuration. → evidence + case
- The faithful builder uses GitHub's release-API asset endpoint because the Dockerfile's browser-download URL returned 404 from the FETCH sandbox; the selected version, architecture name, and published digest remain the same. → case
- GitHub release metadata is volatile, so its two FETCH work trees use TOFU consumed pins rather than `EXPECT`; the asset's published digest remains verified with `sha256sum --check`. The 2026-08-05 update probe found each FETCH stable across its two reads and the pinned cold replay passed. → evidence
