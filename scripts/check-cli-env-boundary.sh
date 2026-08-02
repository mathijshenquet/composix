#!/usr/bin/env bash
set -euo pipefail

# CIP-90: CIX configuration belongs to clap derives at the CLI boundary.
matches=$(rg -n 'std::env::var(_os)?\("CIX_|env::var(_os)?\("CIX_|env::(set_var|remove_var)' crates \
  --glob '*.rs' \
  --glob '!crates/cix-compose/**' || true)
if [ -n "$matches" ]; then
  printf '%s\n' "$matches"
  exit 1
fi

# cix-compose remains a CIP-90 leg-B allowlist while track/netns owns it.
