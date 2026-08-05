# watchtower migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode and live socket runtime were intentionally not run.

- `target/debug/cix build .#watchtower` — synchronous exit 0.
- `target/debug/cix build --cold .#watchtower` — synchronous exit 0, `/nix/store/p54rv14149jh8r2i5h677snc5786fgia-cix-item-watchtower`.
- `target/debug/cix inspect /nix/store/p54rv14149jh8r2i5h677snc5786fgia-cix-item-watchtower` confirms TCP 8080, egress, `start: ["/watchtower"]`, and read-only `/var/run/docker.sock`.

The manifest is desk evidence only; the gate has no dockerd for a real bridge probe.
