# VM dogfood check

Run the disposable NixOS VM test with:

```console
nix flake check
# or
nix build .#checks.x86_64-linux.vm-dogfood
```

For the inner loop, classify the changed paths against each scenario's declared
product contracts and run only the intersecting scenarios:

```console
nix run .#progressive-vm-check
# or choose the comparison point explicitly
nix run .#progressive-vm-check -- --base origin/main
# show the derived selection without starting VMs
nix run .#progressive-vm-check -- --dry-run
# reproduce the conservative leg-1 derivation selector
nix run .#progressive-vm-check -- --selector old --dry-run
# release-grade escape hatch
nix run .#progressive-vm-check -- --full
```

The contract inventory is `scenario-contracts.json`. Its ordered path rules
map product code to contract surfaces, and every scenario declares the surfaces
it proves. Scenario files key themselves; the shared harness, package metadata,
and other cross-cutting inputs key every scenario. A new or unclassified product
path also selects every scenario, while explicitly non-product and non-VM
surfaces explain why they select none. The selector validates that every tracked
Rust product input is classified and prints every change, selected scenario, and
skipped scenario with its reason. `--full` remains the unchanged complete
`nix flake check -L` matrix.

`--target <commit>` reproduces a historical commit against its first parent;
`--base <commit>` overrides either comparison point. `--rebuild` asks Nix to
rerun selected derivations for measurement. Normal selected builds are capped at
two concurrent Nix jobs because each scenario boots a QEMU VM.

The test runs the nginx and PostgreSQL examples as root through `cix`, verifies their HTTP and
TCP interfaces, confirms `cix ps` reports each service, stops them, and checks that no cix units
remain. The demo outputs are built as part of the host-side test closure; the VM has no network
and does not invoke Nix. Expect a few minutes on a cold cache and roughly a minute when cached.
