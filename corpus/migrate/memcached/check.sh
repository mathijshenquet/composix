#!/usr/bin/env bash
set -euo pipefail

mode=${1:-}
root=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ ! -d $root/context ]]; then
  echo 'context/ missing — run ../fetch.sh memcached first' >&2
  exit 1
fi
cix=${CIX:-"$root/../../../target/debug/cix"}
docker_tag=migrate-memcached-docker
container=
unit=

cleanup() {
  if [[ -n $container ]]; then docker stop "$container" >/dev/null 2>&1 || true; fi
  if [[ -n $unit ]]; then sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

probe() {
  local port=$1 attempt result
  for attempt in $(seq 1 10); do
    result=$(printf 'version\r\n' | timeout 2 nc 127.0.0.1 "$port" || true)
    if [[ $result == VERSION\ * ]]; then printf '%s\n' "$result"; return 0; fi
    sleep 1
  done
  return 1
}

cd "$root"
case $mode in
  docker)
    image=$(timeout 360 docker build --quiet --file "$root/Dockerfile" --tag "$docker_tag" "$root/context")
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --publish 127.0.0.1:11211:11211 "$docker_tag")
    probe 11211
    printf 'PASS docker\n'
    ;;
  cix)
    item=$(timeout 240 "$cix" build "$root#memcached")
    printf 'cix item %s\n' "$item"
    unit=$(timeout 30 sudo -n "$cix" run --detach "$item")
    printf 'cix unit %s\n' "$unit"
    probe 11211
    printf 'PASS cix\n'
    ;;
  *) echo 'usage: ./check.sh {docker|cix}' >&2; exit 1 ;;
esac
