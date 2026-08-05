Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The docker-socket bridge declares `/var/run/docker.sock:ro`; a real activation needs `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`. It is desk evidence only: no gate dockerd exists. Journald/`cix logs` and Nix pins/`cix` updates are the native answers; the bridge demonstrates coexistence, not migration. → evidence
- The available `oryxBuildBinary` is runnable but does not prove byte identity with Docker's absent CI-provided `watchtower` payload. → case
