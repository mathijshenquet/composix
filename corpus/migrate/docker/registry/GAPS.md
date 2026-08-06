Generated: migrate.md@current · track-expand-postgres-registry · 2026-08-06
Status: current

- The upstream image uses `tonistiigi/xx` and a build-platform Go stage to cross-compile for `TARGETPLATFORM`. The faithful translation intentionally targets the native host architecture, so the cross-compile layer and multi-platform artifact claim dissolve. → case
- The Go module graph is fetched by an explicit pinned-by-consumption `go mod download` FETCH; the upstream Dockerfile's cache mounts and Docker image bases are not part of the Cix closure. → evidence
- A warm faithful build completes, but `cix build --cold` stops synchronously at `FETCH go mod download`: the warm read set contains `.cache/go-mod` while the cold read set has it absent. The current Cixfile keeps the real warm case and records the cold replay wall rather than claiming cold compatibility. → evidence
- The development config's filesystem storage root is declared as the service's durable `/var/lib/registry` role, while Docker's Alpine layout and image metadata dissolve into the locked runtime. → case
