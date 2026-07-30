#!/usr/bin/env bash
set -euo pipefail

mode=${1:-}
root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../target/debug/cix"}
docker_tag=migrate-echo-server-docker
container=
unit=

cleanup() {
  if [[ -n $container ]]; then docker stop "$container" >/dev/null 2>&1 || true; fi
  if [[ -n $unit ]]; then sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

probe() {
  local url=$1 attempt
  for attempt in $(seq 1 10); do
    if curl --fail --silent --max-time 2 "$url" | grep -q '"http"'; then return 0; fi
    sleep 1
  done
  return 1
}

cd "$root"
case $mode in
  docker)
    image=$(timeout 240 docker build --quiet --tag "$docker_tag" .)
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --publish 127.0.0.1:18081:80 "$docker_tag")
    probe http://127.0.0.1:18081/
    printf 'PASS docker\n'
    ;;
  cix)
    build_output=$(timeout 20 "$cix" build . | tee /dev/stderr)
    item=$(printf '%s\n' "$build_output" | tail -n 1)
    unit=$(timeout 30 sudo -n "$cix" run --detach "$item")
    printf 'cix unit %s\n' "$unit"
    probe http://127.0.0.1:80/
    printf 'PASS cix\n'
    ;;
  *) echo 'usage: ./check.sh {docker|cix}' >&2; exit 1 ;;
esac
