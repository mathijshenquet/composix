# track/dirs2 fix round — scenario-dirs2 red on the independent gate

The independent gate re-run FAILED `checks.x86_64-linux.scenario-dirs2`
on your committed branch (your LOG claims it green — investigate why
your run reported a pass; record the answer honestly in the LOG).

Two observed defects, from the gate transcript:

1. Every fixture unit crash-loops:
   `bin/start: line 4: sleep: command not found` — the start scripts
   reference bare `sleep` but the unit environment cannot resolve it.
   Fix properly (absolute coreutils path or a correct PATH via the
   manifest), and make the scenario assert the units are active AND
   stay active (a crash-looping unit momentarily reporting active must
   not pass).
2. `machine.succeed("test -f /tmp/dirs2/host-state/host-state
   /tmp/dirs2/host-media/host-media")` — `test -f A B` is a shell
   syntax error ("binary operator expected"), it can never succeed.
   Split into separate assertions. Audit the whole scenario for any
   other assertion that cannot fail or cannot pass.

Then: re-run `nix build .#checks.x86_64-linux.scenario-dirs2 --no-link
-L` until green, and finish with the FULL
`devenv shell -- nix flake check -L`. Exact repros in
crates/cix-compose/LOG.md. Commit on this branch when green.
