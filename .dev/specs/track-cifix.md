# track/cifix — de-host the artifact_kinds fixture (last CI-red cause)

Read AGENTS.md first. Context: CI's `test` job fails on
`app_propagates_exit_status` (crates/cix/tests/artifact_kinds.rs): the fixture
resolves `sh` from host PATH; on the CI runner that is `/usr/bin/sh`, which is
not a store path, and cix correctly refuses non-store exec. On dev machines
devenv happens to put a store sh on PATH, masking it. Scope: that test file
only.

1. Make the fixture host-independent: if the PATH-resolved `sh` is not under
   /nix/store, obtain a store shell (e.g. `nix build --print-out-paths
   --no-link nixpkgs#bash` — acceptable ambient registry use inside a test
   guard; note it in a comment) and use `<out>/bin/sh`. If that also fails,
   extend the existing skip guard with an honest message — never assert
   against a non-store shell.
2. Verify BOTH paths: the test passes locally (store sh via devenv), and
   passes with PATH stripped of store shells (simulate CI:
   `PATH=/usr/bin:/bin cargo test -p cix --test artifact_kinds` — document
   the exact repro and result in the LOG).
3. Gate: fmt / warning-denied clippy / workspace tests / the point-2 repro.
   Exact repros in crates/cix/LOG.md. Commit on this branch when green.
