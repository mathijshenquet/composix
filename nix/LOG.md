# VM dogfood log

- 2026-07-28: Started the VM dogfood track. The existing nginx and PostgreSQL examples are standalone Nix functions and their specifications already use only store paths, so the VM test will import them with its host `pkgs` and run those outputs directly.
- 2026-07-28: Added the minimal flake, VM test, and runbook. Pinned nixpkgs at `624af665418d3c65d544145b4d34ad696439570e`; `nix flake show --no-write-lock-file` evaluates the package and VM check successfully.
- 2026-07-28: First package build reached the Rust checks, which fail because the existing `cix` tour tests shell out to `nix` and the pure `buildRustPackage` sandbox does not provide it. Set `doCheck = false` for this binary package; the VM check is the integration verification required by this track.
