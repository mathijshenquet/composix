Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- A useful Watchtower runtime requires Docker's host socket and control API, which composix deliberately refuses. → refused
- The Dockerfile consumes a CI-provided `watchtower` binary absent from the recorded repository context; compiling the supplied Go source modernizes the build side but is a different supply contract. → evidence
- Docker's exec healthcheck has no equivalent generic exec health probe, and no weaker native HTTP/TCP assertion is invented. → language (exec health probe)
- A cold audit reports a warm/cold Go build-cache read-set mismatch, so source-build cold stability remains unproved. → case (cold stability)
- Repeating a direct `COPY` destination against an already populated warm builder root is rejected; the exact product finding is promoted in `cips/open-questions.md`. → language (warm-root duplicate COPY)
