Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The docker-socket bridge declares `/var/run/docker.sock:ro`; activation requires `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`. It is desk evidence only: no gate dockerd exists. Journald/`cix logs` and Nix pins/`cix` updates are the native answers; this bridge demonstrates coexistence, not migration. → evidence
- `pnpm fetch --ignore-scripts` exceeded the independently applied 300-second update-lock bound, so no frontend artifact or runtime item is claimed. → case
