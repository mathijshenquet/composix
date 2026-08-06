Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The docker-socket bridge declares `/var/run/docker.sock:ro`; activation requires `--dir /var/run/docker.sock=host:/var/run/docker.sock --identity <stable-identity>`. It is desk evidence only: no gate dockerd exists. Journald/`cix logs` and Nix pins/`cix` updates are the native answers; this bridge demonstrates coexistence, not migration. → evidence
- The former pnpm hang was missing TLS trust: without `cacert` the clean FETCH timed out after 180 seconds with no CAS files; with `${pkgs.cacert}` the actual Cix FETCH completes both stability probes. → case
- The frozenStore recheck uses pinned nixpkgs pnpm 11.18.0 (upstream pins the already-eligible 11.17.0) and seals the complete 20,175-file CAS plus `index.db`; no store bytes are stripped, normalized, or regenerated. The downstream offline frozen install stayed active under the read tracer until the 1,200-second bound, so no frontend, item, or runtime is claimed and no lock update is retained. → evidence
