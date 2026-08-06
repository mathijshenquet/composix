Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The docker-socket bridge declares `/var/run/docker.sock:ro`; activation requires `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`. It is desk evidence only: no gate dockerd exists. Journald/`cix logs` and Nix pins/`cix` updates are the native answers; this bridge demonstrates coexistence, not migration. → evidence
- The former pnpm hang was missing TLS trust: without `cacert` the clean FETCH timed out after 180 seconds with no CAS files; with `${pkgs.cacert}` the actual cix FETCH completed both stability probes and exited 0. The full frontend, item, and runtime remain unverified. → case
