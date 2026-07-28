# VM dogfood check

Run the disposable NixOS VM test with:

```console
nix flake check
# or
nix build .#checks.x86_64-linux.vm-dogfood
```

The test runs the nginx and PostgreSQL examples as root through `cix`, verifies their HTTP and
TCP interfaces, confirms `cix ps` reports each service, stops them, and checks that no cix units
remain. The demo outputs are built as part of the host-side test closure; the VM has no network
and does not invoke Nix. Expect a few minutes on a cold cache and roughly a minute when cached.
