Generated: migrate.md@c43ae9b · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- `START caddy respond` is a probe-shaped toy, not the upstream `caddy run --config /etc/caddy/Caddyfile --adapter caddyfile` contract; the upstream Caddyfile and welcome page are absent. → case
- Only port 8080 is declared: upstream HTTP/HTTPS/admin ports 80, 443, and 2019 are not represented. → case
- Upstream also exposes QUIC on 443/UDP, while `PORT` has no protocol spelling for a faithful declaration. → language (candidate: UDP ports)
- `/config` and `/data` were collapsed below one `/var/lib/caddy` state role even though role-directory paths can now mirror both upstream paths directly. → prompt
- The package binary is exposed through an implicit `/bin` link instead of a declared artifact tool import. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The receipt proves only the toy HTTP responder and does not compare the selected nixpkgs Caddy version with upstream 2.11.4. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
