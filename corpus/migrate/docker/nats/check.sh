#!/usr/bin/env bash
set -euo pipefail

mode=${1:-}
root=$(cd -- "$(dirname -- "$0")" && pwd)
cix=${CIX:-"$root/../../../../target/debug/cix"}
docker_tag=migrate-nats-docker
container=
unit=
source_dir=

cleanup() {
  if [[ -n $container ]]; then
    docker stop "$container" >/dev/null 2>&1 || true
  fi
  if [[ -n $unit ]]; then
    sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true
  fi
  if [[ -n $source_dir ]]; then
    rm -rf "$source_dir"
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
    source_dir=$(mktemp -d)
    timeout 60 curl --fail --location --silent --show-error https://github.com/nats-io/nats-docker/archive/refs/heads/main.tar.gz --output "$source_dir/source.tar.gz"
    tar -xf "$source_dir/source.tar.gz" -C "$source_dir"
    image=$(timeout 240 docker build --quiet --tag "$docker_tag" "$source_dir/nats-docker-main/2.12.x/alpine3.22")
    printf 'docker image %s\n' "$image"
    container=$(timeout 30 docker run --detach --rm --publish 127.0.0.1:18222:8222 "$docker_tag")
    probe http://127.0.0.1:18222/healthz
    printf 'PASS docker\n'
    ;;
  cix)
    item=$(timeout 240 "$cix" build "$root#nats")
    printf 'cix item %s\n' "$item"
    unit=$(timeout 30 sudo -n "$cix" run --detach "$item")
    printf 'cix unit %s\n' "$unit"
    probe http://127.0.0.1:8222/healthz
    printf 'PASS cix\n'
    ;;
  *)
    echo "usage: $0 {docker|cix}" >&2
    exit 1
    ;;
esac
