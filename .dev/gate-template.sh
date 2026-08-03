#!/usr/bin/env bash
# Independent gate template — instrumented + resource-bounded.
# Usage: gate-template.sh <worktree-dir> <out-prefix>
WT="$1"; PREFIX="$2"
cd "$WT" || exit 1
PARSER=/tmp/claude-1001/-home-mathijs-composix/66ceef54-dc8d-437a-8148-fcdf659fca6f/scratchpad/nix-time-parse.py

step() { echo "=== STEP: $1 ==="; }

step fmt
devenv shell -- cargo fmt --all --check
if [ $? -ne 0 ]; then echo "STEP-FAILED: fmt"; exit 1; fi

step cixfmt-examples
devenv shell -- cargo run -- fmt --check examples
if [ $? -ne 0 ]; then echo "STEP-FAILED: cixfmt-examples"; exit 1; fi

step clippy
devenv shell -- cargo clippy --workspace --all-targets -- -D warnings
if [ $? -ne 0 ]; then echo "STEP-FAILED: clippy"; exit 1; fi

step test-workspace
devenv shell -- cargo test --workspace
if [ $? -ne 0 ]; then echo "STEP-FAILED: test-workspace"; exit 1; fi

step tour-regen
devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour
if [ $? -ne 0 ]; then echo "STEP-FAILED: tour-regen"; exit 1; fi

step tour-drift
git diff --exit-code -- docs/tour
if [ $? -ne 0 ]; then echo "STEP-FAILED: tour-drift"; exit 1; fi

step flake-check
# Bounded so the gate never starves interactive work (32-thread host).
nice -n 10 devenv shell -- nix flake check -j6 --cores 4 -L 2>&1 | tail -40
rc=${PIPESTATUS[0]}
if [ $rc -ne 0 ]; then echo "STEP-FAILED: flake-check"; exit 1; fi

echo "GATE-ALL-GREEN"
