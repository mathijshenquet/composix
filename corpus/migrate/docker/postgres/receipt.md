# PostgreSQL migration receipt

2026-08-06 synchronous receipts. `bash corpus/migrate/fetch.sh
docker/postgres` fetched `docker-library/postgres` at
`62a714f93cc32220de46fd12235c9d509e3b1ad6`, with the `17/trixie` context.

`devenv shell -- target/debug/cix build --update-lock=vendor
corpus/migrate/docker/postgres#postgres` exited 0 and produced
`/nix/store/if5hb96kxfc79wccd6jsm5a6prh2l19x-cix-item-postgres`. The vendor
FETCH downloaded gosu 1.19 and verified its upstream SHA-256 inside the
network step. The dissolved twin build and both faithful and dissolved
`--cold` builds exited 0.

`devenv shell -- env CIX="$PWD/target/debug/cix" ./check.sh cix` was run
synchronously and did not reach `pg_isready`: the probe timed out after
repeated connection refusals. Direct journal inspection records the exact
startup wall: `/var/run/postgresql` cannot be chmodded under the realized
state role, then PostgreSQL reports its compiled package `lib` directory is
not available inside the isolated item. The known arbitrary-path state-role
defect is cited in `GAPS.md`; this is not graded as runtime green.
