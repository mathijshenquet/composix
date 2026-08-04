Generated: migrate.md@4e76da0 · unknown · 2026-08-02
Status: current

- The checked conversion substitutes Python/nginx shape stubs for Mastodon's web, Sidekiq, and streaming applications and adds a sixth cleanup member; it proves composition mechanisms, not a Mastodon application migration. → case
- Only `compose.json` is visible in the browser even though `corpus-mastodon-*:checked` tags are produced from six member Cixfiles by `check.sh`; surface that provenance and the member sources. → browser
- The upstream external/internal multi-network split and `internal: true` egress boundary remain unexpressed. → language ([D26/D27 named-network frontier](../../../docs/design.md#building-now-updated-2026-08-02))
- `.env.production` and its application configuration are replaced by one database credential and stub defaults; the unconverted configuration surface must remain explicit. → case
- HTTP/TCP liveness loses its resident pinger on systemd 257 even though the accepted VM passes on newer systemd. → evidence
