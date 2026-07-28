#!/usr/bin/env bash

repo_root() {
  git -C "$(dirname -- "${BASH_SOURCE[0]}")" rev-parse --show-toplevel
}

resolve_cix() {
  local root candidate
  root=$(repo_root)
  if [[ -n ${CIX_BIN:-} ]]; then
    candidate=$CIX_BIN
  elif command -v cix >/dev/null 2>&1; then
    candidate=$(command -v cix)
  else
    candidate="$root/target/debug/cix"
  fi
  if [[ ! -x $candidate ]]; then
    devenv shell -- cargo build --quiet -p cix
  fi
  realpath "$candidate"
}

wait_for_path() {
  local path=$1
  for _ in {1..100}; do
    [[ -e $path ]] && return 0
    sleep 0.1
  done
  echo "timed out waiting for $path" >&2
  return 1
}

wait_for_unit_gone() {
  local unit=$1
  for _ in {1..100}; do
    if ! sudo systemctl show "$unit" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  echo "timed out waiting for $unit to be collected" >&2
  return 1
}

stop_unit() {
  local unit=${1:-}
  [[ -z $unit ]] && return 0
  sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  sudo systemctl reset-failed "$unit" >/dev/null 2>&1 || true
}

assert_property() {
  local unit=$1 property=$2 expected=$3 actual
  actual=$(sudo systemctl show "$unit" --property="$property" --value)
  if [[ $actual != "$expected" ]]; then
    echo "$unit: expected $property=$expected, got $actual" >&2
    return 1
  fi
}

