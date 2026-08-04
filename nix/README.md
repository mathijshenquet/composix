# VM dogfood check

Run the disposable NixOS VM test with:

```console
nix flake check
# or
nix build .#checks.x86_64-linux.vm-dogfood
```

For the inner loop, compare VM check derivations with the preceding commit and
run only the changed roots:

```console
nix run .#progressive-vm-check
# or choose the comparison point explicitly
nix run .#progressive-vm-check -- --base origin/main
# show the derived selection without starting VMs
nix run .#progressive-vm-check -- --dry-run
# release-grade escape hatch
nix run .#progressive-vm-check -- --full
```

The selector evaluates both trees and derives its list from the scenario
derivation paths. It prints every selected and skipped scenario; `--full` is
the unchanged complete `nix flake check -L` matrix.

The test runs the nginx and PostgreSQL examples as root through `cix`, verifies their HTTP and
TCP interfaces, confirms `cix ps` reports each service, stops them, and checks that no cix units
remain. The demo outputs are built as part of the host-side test closure; the VM has no network
and does not invoke Nix. Expect a few minutes on a cold cache and roughly a minute when cached.
