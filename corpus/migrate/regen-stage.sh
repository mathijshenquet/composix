#!/usr/bin/env bash
# Build a COLD staging directory for a corpus-case regeneration worker.
# Includes: the teaching prompt, the upstream side, the probe contract, a
# cix binary. Excludes (deliberately): the existing Cixfile/lock, GAPS.md,
# receipt.md — cold regeneration must not see canon (docs/corpus.md loops).
set -euo pipefail

selector=${1:?usage: regen-stage.sh <case>|<axis>/<case> [dest]}
root=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(cd -- "$root/../.." && pwd)
case $selector in
  */*)
    axis=${selector%%/*}
    case_name=${selector#*/}
    [[ -n $axis && -n $case_name && $case_name != */* ]] || { echo "invalid case: $selector" >&2; exit 1; }
    ;;
  *)
    axis=docker
    case_name=$selector
    ;;
esac
case_dir="$root/$axis/$case_name"
qualified_case="$axis/$case_name"
dest=${2:-"$HOME/regen-stage/${axis}-${case_name}"}

[[ -d "$case_dir" ]] || { echo "unknown case: $selector" >&2; exit 1; }
[[ -e "$dest" ]] && { echo "dest exists: $dest (remove it first)" >&2; exit 1; }
mkdir -p "$dest/bin"

cp "$repo/docs/migrate.md" "$dest/MIGRATE.md"
for f in Dockerfile SOURCE check.sh; do
  [[ -f "$case_dir/$f" ]] && cp "$case_dir/$f" "$dest/$f"
done
for f in "$case_dir"/upstream-*; do
  [[ -f "$f" ]] && cp "$f" "$dest/"
done

if [[ ! -d "$case_dir/context" ]]; then
  "$root/fetch.sh" "$qualified_case" || echo "note: no fetched context for $qualified_case" >&2
fi
[[ -d "$case_dir/context" ]] && cp -r "$case_dir/context" "$dest/context"

cix_bin="$repo/target/debug/cix"
[[ -x "$cix_bin" ]] || { echo "build cix first: cargo build -p cix" >&2; exit 1; }
cp "$cix_bin" "$dest/bin/cix"

cat > "$dest/TASK.md" <<'EOF'
# Task: convert this Dockerfile to a Cixfile

Read MIGRATE.md fully first — it is the complete teaching contract.
Then read the Dockerfile, every file under context/ it uses, and
check.sh (the acceptance probe: your conversion must serve the same
behavior it probes; its relative paths refer to the original repo, do
not run its docker side).

Produce in THIS directory:
- `Cixfile` — the Dockerfile-faithful translation.
- `Cixfile.dissolved` — ONLY if MIGRATE.md's dissolution rule applies
  (the app is directly packaged in nixpkgs): the nixpkgs-direct twin.
- Any aux files the Cixfile needs (checked-in scripts/configs).
- `NOTES.md` — your honest gap list: every upstream ENV/config/behavior
  you translated, dissolved (with the reason), or could not carry.

Verify: `./bin/cix build .` must exit 0 (use `--file Cixfile.dissolved`
for the twin). Iterate until green or until you hit a wall you can
explain precisely; a clearly-explained wall in NOTES.md is a valid
outcome, an unexplained failure is not.

Hard rules: work only inside this directory with ./bin/cix; consult
nothing outside it (no repositories, no web fetches beyond what
`cix build` itself performs); do not invent hashes — follow MIGRATE.md's
EXPECT discipline.
EOF

echo "staged: $dest"
