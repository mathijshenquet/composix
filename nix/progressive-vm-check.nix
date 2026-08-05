{ pkgs }:

pkgs.writeShellApplication {
  name = "cix-progressive-vm-check";
  runtimeInputs = [ pkgs.coreutils pkgs.git pkgs.jq pkgs.nix pkgs.python3 ];
  text = ''
    usage() {
      printf '%s\n' 'usage: nix run .#progressive-vm-check -- [--base <commit>] [--target <commit>] [--selector new|old] [--dry-run] [--rebuild] [--full]'
    }

    full=false
    dry_run=false
    rebuild=false
    base=""
    target=""
    selector=new
    while (($#)); do
      case "$1" in
        --base)
          shift
          if (($# == 0)); then
            usage >&2
            exit 2
          fi
          base="$1"
          ;;
        --target)
          shift
          if (($# == 0)); then
            usage >&2
            exit 2
          fi
          target="$1"
          ;;
        --selector)
          shift
          if (($# == 0)); then
            usage >&2
            exit 2
          fi
          selector="$1"
          ;;
        --full)
          full=true
          ;;
        --dry-run)
          dry_run=true
          ;;
        --rebuild)
          rebuild=true
          ;;
        -h|--help)
          usage
          exit 0
          ;;
        *)
          usage >&2
          exit 2
          ;;
      esac
      shift
    done

    if [[ "$selector" != new && "$selector" != old ]]; then
      printf 'unknown selector: %s (expected new or old)\n' "$selector" >&2
      exit 2
    fi

    repo=$(git rev-parse --show-toplevel)
    cd "$repo"
    started=$(date +%s%N)

    if [[ -n "$target" ]]; then
      target=$(git rev-parse --verify "''${target}^{commit}")
      current_ref="git+file://$repo?rev=$target"
      current_label="$target"
    else
      current_ref="path:$repo"
      current_label="$(git rev-parse HEAD)+worktree"
    fi

    if [[ -z "$base" ]]; then
      if [[ -n "$target" ]]; then
        base="$target^"
      else
        base=HEAD^
      fi
    fi
    base=$(git rev-parse --verify "''${base}^{commit}")
    base_ref="git+file://$repo?rev=$base"

    if [[ "$full" == true ]]; then
      printf 'VM selection: full matrix requested for %s.\n' "$current_label"
      if [[ -n "$target" ]]; then
        exec nix flake check -L "$current_ref"
      fi
      exec nix flake check -L
    fi

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    derivations() {
      nix eval --json "$1#checks.x86_64-linux" --apply \
        'checks: builtins.mapAttrs (_: check: check.drvPath) checks'
    }

    derivations "$current_ref" > "$tmpdir/current.json"

    if [[ "$selector" == old ]]; then
      derivations "$base_ref" > "$tmpdir/base.json"
      jq -n \
        --slurpfile current "$tmpdir/current.json" \
        --slurpfile base "$tmpdir/base.json" '
          $current[0] as $currentChecks
          | $base[0] as $baseChecks
          | {
              changes: [],
              selections: [
                $currentChecks
                | keys[]
                | select(startswith("scenario-"))
                | . as $name
                | if $currentChecks[$name] == ($baseChecks[$name] // null)
                  then {status: "skipped", name: $name, reason: "unchanged derivation"}
                  else {status: "selected", name: $name, reason: "derivation changed since base"}
                  end
              ]
            }
        ' > "$tmpdir/selections.json"
    else
      if [[ -n "$target" ]]; then
        git diff --name-only --diff-filter=ACDMRTUXB "$base" "$target" -- \
          > "$tmpdir/changes"
      else
        git diff --name-only --diff-filter=ACDMRTUXB "$base" -- > "$tmpdir/changes"
        git ls-files --others --exclude-standard >> "$tmpdir/changes"
      fi
      LC_ALL=C sort -u "$tmpdir/changes" -o "$tmpdir/changes"
      ${pkgs.python3}/bin/python3 ${./scenario-contracts.py} \
        --contracts ${./scenario-contracts.json} \
        --repo "$repo" \
        --checks "$tmpdir/current.json" \
        select --changes "$tmpdir/changes" > "$tmpdir/selections.json"
    fi

    printf 'VM selection: %s selector comparing %s to %s.\n' "$selector" "$current_label" "$base"
    if [[ "$selector" == new ]]; then
      while IFS=$'\t' read -r path classification; do
        printf 'change: %s (%s)\n' "$path" "$classification"
      done < <(jq -r '.changes[] | [.path, .classification] | @tsv' "$tmpdir/selections.json")
    fi

    selected=()
    while IFS=$'\t' read -r status name reason; do
      printf '%s: %s (%s)\n' "$status" "$name" "$reason"
      if [[ "$status" == selected ]]; then
        selected+=("$current_ref#checks.x86_64-linux.$name")
      fi
    done < <(jq -r '.selections[] | [.status, .name, .reason] | @tsv' "$tmpdir/selections.json")

    selection_finished=$(date +%s%N)
    selection_elapsed=$((selection_finished - started))
    printf 'VM selection: %s scenario derivation(s) selected; selection wall-clock %d.%03ds.\n' \
      "''${#selected[@]}" "$((selection_elapsed / 1000000000))" "$(((selection_elapsed / 1000000) % 1000))"

    if ((''${#selected[@]} == 0)); then
      printf '%s\n' 'VM selection: no scenario contract changed; no VM test ran.'
      exit 0
    fi

    if [[ "$dry_run" == true ]]; then
      exit 0
    fi

    build_args=(-L --no-link --max-jobs 2)
    if [[ "$rebuild" == true ]]; then
      build_args+=(--rebuild)
    fi
    if nix build "''${build_args[@]}" "''${selected[@]}"; then
      status=0
    else
      status=$?
    fi
    finished=$(date +%s%N)
    elapsed=$((finished - started))
    printf 'VM selection: build exit %s; total wall-clock %d.%03ds.\n' \
      "$status" "$((elapsed / 1000000000))" "$(((elapsed / 1000000) % 1000))"
    exit "$status"
  '';
}
