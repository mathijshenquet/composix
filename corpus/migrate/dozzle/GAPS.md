Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- A useful Dozzle runtime requires `/var/run/docker.sock` and Docker's control API, which composix deliberately refuses; `CLAIM egress` is not a substitute. → refused
- `main.go` embeds `shared_cert.pem` and `shared_key.pem`, but the supplied source context lacks both files, so the backend cannot produce an item without fabricating inputs. → evidence
- The frontend FETCH is cold-unstable: replay reports a warm/cold read-set difference at `.` after earlier isolated pnpm runs stalled; do not repin or weaken the graph, and normalize it in the volatile-fetch fix round. → case (cold stability)
- Docker's exact Node/pnpm selection, cross-platform arguments, and configurable `CLOUD_URL` become locked-host packages and an explicit default; custom cross-build/cloud inputs require editing the Cixfile. → case
