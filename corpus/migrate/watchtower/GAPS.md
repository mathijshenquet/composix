Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- A faithful runtime requires Docker's host socket and control API, which composix deliberately refuses. → refused
- The upstream Dockerfile consumes a CI-provided `watchtower` binary absent from the recorded repository context, so its Docker build cannot be reproduced from this pair. → evidence
- The Cix conversion compiles source instead, a different supply contract that must not stand in for a Dockerfile-faithful twin. → case
- Copying the built binary into `/bin` and invoking it by a bare name relies on implicit self-import rather than a declared artifact tool contract. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
