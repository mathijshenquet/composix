#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "$0")" && pwd)

die() {
  printf 'fetch.sh: %s\n' "$*" >&2
  exit 1
}

source_value() {
  local source=$1 key=$2
  sed -nE "s/^${key}:[[:space:]]*(.+)[[:space:]]*$/\\1/p" "$source" | head -n 1
}

check_relative_path() {
  local path=$1 label=$2
  [[ -n $path && $path != /* && $path != *'..'* ]] || die "$label must be a relative path"
}

fetch() {
  local name candidate source repo rev context_path temp source_tree excluded

  name=$1
  candidate="$root/$name"
  source="$root/$name/SOURCE"

  [[ -d $candidate ]] || die "unknown candidate '$name'"
  [[ -f $source ]] || die "$name has no SOURCE file"

  repo=$(source_value "$source" 'Repository')
  if [[ -z $repo ]]; then
    repo=$(sed -nE 's|^Dockerfile and build context:[[:space:]]*(https://[^ ,`]+\.git).*|\1|p' "$source" | head -n 1)
  fi
  rev=$(sed -nE 's/^Resolved revision:[[:space:]]*([0-9a-f]{40})[[:space:]]*$/\1/p' "$source" | head -n 1)
  context_path=$(source_value "$source" 'Context path')

  [[ $repo =~ ^https://.+\.git$ ]] || die "$name SOURCE lacks a parseable repository URL"
  [[ $rev =~ ^[0-9a-f]{40}$ ]] || die "$name SOURCE lacks a parseable resolved revision"
  [[ -n $context_path ]] || die "$name SOURCE does not declare a context path"
  check_relative_path "$context_path" "context path for $name"

  temp=$(mktemp -d "${TMPDIR:-/tmp}/composix-fetch.${name}.XXXXXX")
  trap 'rm -rf -- "$temp"' RETURN
  git clone --quiet --no-checkout "$repo" "$temp"
  git -C "$temp" checkout --quiet --detach "$rev"
  if [[ $context_path == . ]]; then
    source_tree=$temp
  else
    source_tree="$temp/$context_path"
  fi
  [[ -d $source_tree ]] || die "$name context path '$context_path' is absent at $rev"

  while IFS= read -r excluded; do
    [[ -z $excluded ]] && continue
    check_relative_path "$excluded" "excluded context path for $name"
    rm -rf -- "$source_tree/$excluded"
  done < <(sed -nE 's/^Exclude context path:[[:space:]]*(.+)[[:space:]]*$/\1/p' "$source")
  find "$source_tree" -depth -type d -name .git -exec rm -rf -- {} +

  rm -rf -- "$candidate/context"
  mv "$source_tree" "$candidate/context"
  printf 'fetched %s at %s\n' "$name" "$rev"
}

case ${1:-} in
  --all)
    [[ $# -eq 1 ]] || die 'usage: ./fetch.sh [--all|<name>]'
    while IFS= read -r source; do
      if grep -q '^Context path:' "$source"; then
        fetch "$(basename "$(dirname "$source")")"
      fi
    done < <(find "$root" -mindepth 2 -maxdepth 2 -type f -name SOURCE | sort)
    ;;
  '') die 'usage: ./fetch.sh [--all|<name>]' ;;
  *)
    [[ $# -eq 1 ]] || die 'usage: ./fetch.sh [--all|<name>]'
    fetch "$1"
    ;;
esac
