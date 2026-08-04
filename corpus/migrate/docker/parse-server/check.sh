#!/usr/bin/env bash
set -euo pipefail

mode=${1:?usage: ./check.sh docker|cix}
root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
name=migrate-r5-parse-server
network=$name-net
mongo=$name-mongo
container=
unit=

cleanup() {
  [[ -z $unit ]] || sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
  [[ -z $container ]] || docker rm -f "$container" >/dev/null 2>&1 || true
  docker rm -f "$mongo" >/dev/null 2>&1 || true
  docker network rm "$network" >/dev/null 2>&1 || true
}
trap cleanup EXIT

probe() {
  for _ in $(seq 1 60); do
    if curl --fail --silent --max-time 2 http://127.0.0.1:18091/parse/health | grep -q '"status":"ok"'; then
      return 0
    fi
    sleep 1
  done
  return 1
}

[[ -d $root/context ]] || { echo 'context/ missing — run ../../fetch.sh parse-server first' >&2; exit 1; }
cd "$root"
docker network create "$network" >/dev/null
case $mode in
  docker)
    docker run --detach --rm --name "$mongo" --network "$network" mongo:8.0.4 >/dev/null
    image=$(timeout 1200 docker build --quiet --file Dockerfile --tag "$name" context)
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --name "$name" --network "$network" \
      --publish 127.0.0.1:18091:1337 "$name" \
      --appId migration-check --masterKey migration-secret \
      --databaseURI "mongodb://$mongo:27017/test" \
      --serverURL http://localhost:1337/parse)
    probe
    printf 'PASS docker\n'
    ;;
  cix)
    docker run --detach --rm --name "$mongo" --network "$network" \
      --publish 127.0.0.1:27027:27017 mongo:8.0.4 >/dev/null
    item=$(timeout 1200 "$cix" build .#parse-server)
    printf 'cix item %s\n' "$item"
    unit=$(timeout 30 sudo -n "$cix" run --detach \
      -e PARSE_SERVER_APPLICATION_ID=migration-check \
      -e PARSE_SERVER_MASTER_KEY=migration-secret \
      -e PARSE_SERVER_DATABASE_URI=mongodb://127.0.0.1:27027/test \
      -e PARSE_SERVER_URL=http://localhost:18091/parse "$item" | tail -n1)
    printf 'cix unit %s\n' "$unit"
    probe
    printf 'PASS cix\n'
    ;;
  *) echo 'usage: ./check.sh docker|cix' >&2; exit 1 ;;
esac
