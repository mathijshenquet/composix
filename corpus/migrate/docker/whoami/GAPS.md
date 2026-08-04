Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- The supplied pair has no checked-out Go build context, and the attempted source reconstruction hit Cix's context-free `Not a directory` builder failure; the primary file therefore uses the nixpkgs binary in the Docker runtime layout rather than proving the upstream source build. → evidence
- Docker's scratch-root identity dissolves into Cix's isolated dynamic service identity; fixed root ownership is not preserved. → case
