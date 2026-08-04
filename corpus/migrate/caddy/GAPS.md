Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: current

- Caddy's upstream `/config/caddy` autosave cannot use `CONFIGDIR`: build accepts it, but run rejects configuration roles outside `/etc`, and an `/etc` role would overlap the immutable Caddyfile. The conversion is therefore forced to fold `XDG_CONFIG_HOME` into `/data`; this is the verified [CONFIGDIR product defect](../../../docs/open-questions.md#configdir-is-not-path-free-regen-wave-1-lunas-caddy-verified-2026-08-04), not a preferred layout. → language (CONFIGDIR path freedom)
- The service carries a minimal `/etc/hosts` `FILE` because the sandbox supplies no `localhost`, unlike Docker; remove it only when the verified [sandbox hosts defect](../../../docs/open-questions.md#no-localhost-in-the-service-sandbox-same-wave-caddy) is resolved. → language (service localhost)
- The faithful twin preserves the upstream configuration/assets and all four exposed sockets, but normalizes HTTP from `:80` to `:8080` so the existing Cix probe can bind directly without Docker's host-port remap. → case
