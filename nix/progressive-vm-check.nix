{ pkgs }:

pkgs.writeShellApplication {
  name = "cix-progressive-vm-check";
  runtimeInputs = [ pkgs.git pkgs.jq pkgs.nix ];
  text = ''
    usage() {
      printf '%s\n' 'usage: nix run .#progressive-vm-check -- [--base <commit>] [--dry-run] [--full]'
    }

    full=false
    dry_run=false
    base=""
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
        --full)
          full=true
          ;;
        --dry-run)
          dry_run=true
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

    repo=$(git rev-parse --show-toplevel)
    cd "$repo"

    if [[ "$full" == true ]]; then
      printf '%s\n' 'VM selection: full matrix requested.'
      exec nix flake check -L
    fi

    if [[ -z "$base" ]]; then
      base=HEAD^
    fi
    base=$(git rev-parse --verify "''${base}^{commit}")
    current=$(git rev-parse HEAD)
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    derivations() {
      nix eval --json "$1#checks.x86_64-linux" --apply \
        'checks: builtins.mapAttrs (_: check: check.drvPath) checks'
    }

    derivations "path:$repo" > "$tmpdir/current.json"
    derivations "git+file://$repo?rev=$base" > "$tmpdir/base.json"

    mapfile -t selections < <(
      jq -nr --slurpfile current "$tmpdir/current.json" --slurpfile base "$tmpdir/base.json" '
        $current[0] as $currentChecks
        | $base[0] as $baseChecks
        | $currentChecks
        | keys[]
        | select(startswith("scenario-"))
        | . as $name
        | if $currentChecks[$name] == ($baseChecks[$name] // null)
          then "skipped\t\($name)\tunchanged derivation"
          else "selected\t\($name)\tderivation changed since base"
          end
      ' | sort
    )

    printf 'VM selection: comparing %s to %s.\n' "$current" "$base"
    selected=()
    for selection in "''${selections[@]}"; do
      IFS=$'\t' read -r status name reason <<< "$selection"
      printf '%s: %s (%s)\n' "$status" "$name" "$reason"
      if [[ "$status" == selected ]]; then
        selected+=("path:$repo#checks.x86_64-linux.$name")
      fi
    done

    if ((''${#selected[@]} == 0)); then
      printf '%s\n' 'VM selection: no scenario derivations changed; no VM test ran.'
      exit 0
    fi

    if [[ "$dry_run" == true ]]; then
      printf 'VM selection: %s scenario derivation(s) would run.\n' "''${#selected[@]}"
      exit 0
    fi

    nix build -L --no-link "''${selected[@]}"
  '';
}
