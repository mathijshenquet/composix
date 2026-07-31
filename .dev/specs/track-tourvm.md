# track/tourvm — tours show only the supported system-mode path

Read AGENTS.md first. Mathijs (2026-07-31): "haal die user mode uit de tours,
dat is toch niet zo mooi." SEQUENCED AFTER track/argvenv merges (argvenv sweeps
the same tour files for GRANT/STATEDIR).

## Problem
Tour chapters 03, 05, 06 start services with `cix run --user` and print the
degraded-mode warning three times. The tours are the product's face; they must
show the real thing: system manager, DynamicUser, full hardening profile. The
`--user` flag exists in the transcripts only because the harness
(`crates/cix/tests/tour.rs`) runs under plain `cargo test` without root.

## Shape
Move generation of the run/debug/exec transcript sections into a NixOS VM (the
`nix/vm-dogfood.nix` pattern — a testScript driving the real `cix` against the
system manager as root), so the printed commands become `cix run tour-app:v1
--detach` with genuine hardened system-mode output. Latitude on mechanics, but
respect:

1. **Executability stays honest** — every printed transcript is produced by
   actually running the command; no hand-written output. Drift-check and
   determinism-twice remain gate steps (a VM-produced chapter re-generated
   twice must be byte-identical after the existing NONCE/store-path scrubbing).
2. **Network constraint**: NixOS test VMs have no network. Build steps with
   FETCH cannot run in the VM. Split accordingly: builds/FETCH happen host-side
   as today; the built items are passed into the VM (store closure into the VM
   nix store) and only run/debug/exec transcripts are captured in-VM. Chapters
   01/02/04 (language/build) need not move.
3. The generator needs to be a nix-buildable artifact the VM can execute
   (options: promote the tour generator to a bin target the flake builds, or
   `cargo test --no-run` inside the nix build — pick the least clever one that
   works; document the regen command in the chapter headers).
4. `--user` disappears from every tour page and every degraded-mode warning
   disappears with it. The `--user` feature itself is NOT removed from the
   product (its future is a separate open design: the cix-machine/VM user
   story). docs may still MENTION `--user` as the degraded dev path where
   honest, but no tour transcript uses it.
5. Regen UX: one documented command regenerates everything (host part + VM
   part) and `git diff` shows the result; CI/gate runs the same and fails on
   drift.

## Gate
fmt / warning-denied clippy / workspace tests, tour regen + zero-drift +
determinism-twice (including the VM-generated chapters), vm-dogfood, and the
scenario tier untouched. Exact repros in crates/cix/LOG.md (this track's LOG).
