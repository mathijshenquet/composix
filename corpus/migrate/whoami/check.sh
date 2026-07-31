#!/usr/bin/env bash
set -euo pipefail

mode=${1:-}
root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../target/debug/cix"}
docker_tag=migrate-whoami-docker
container=
unit=

cleanup() {
  if [[ -n $container ]]; then
    docker stop "$container" >/dev/null 2>&1 || true
  fi
  if [[ -n $unit ]]; then
    sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT
trap 'exit 1' ERR

probe() {
  local url=$1
  local attempt
  for attempt in $(seq 1 30); do
    if curl --fail --silent --max-time 2 "$url"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

cd "$root"
case $mode in
  docker)
    image=$(timeout 240 docker build --quiet --tag "$docker_tag" https://github.com/traefik/whoami.git)
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --publish 127.0.0.1:18080:80 "$docker_tag")
    probe http://127.0.0.1:18080/
    printf 'PASS docker\n'
    ;;
  cix)
    item=$(timeout 240 "$cix" build "$root#whoami")
    printf 'cix item %s\n' "$item"
    unit=$(timeout 30 sudo -n "$cix" run --detach "$item")
    printf 'cix unit %s\n' "$unit"
    probe http://127.0.0.1:80/
    printf 'PASS cix\n'
    ;;
  *)
    echo "usage: $0 {docker|cix}" >&2
    exit 1
    ;;
esac
