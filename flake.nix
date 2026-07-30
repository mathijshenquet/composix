{
  description = "composix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      sdbisectPkgs = import nixpkgs {
        inherit system;
        overlays = [
          (final: prev: {
            systemd = prev.systemd.overrideAttrs (old: {
              patches = (old.patches or [ ]) ++ [ ./nix/patches/systemd-6431c34b8a84-revert.patch ];
            });
          })
        ];
      };
      cix = pkgs.rustPlatform.buildRustPackage {
        pname = "cix";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "-p" "cix" ];
        doCheck = false;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postInstall = ''
          wrapProgram "$out/bin/cix" \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.bubblewrap pkgs.nix ]}
        '';
      };
      composixLib = import ./nix/lib.nix { inherit pkgs; };
      withSpecRedis = import ./examples/pack/redis {
        inherit pkgs;
        composix = composixLib;
      };
    in
    {
      packages.${system} = {
        cix = cix;
        withSpecRedis = withSpecRedis;
        # On-demand diagnostic harness (compiles a patched systemd) — deliberately a
        # package, not a check, so `nix flake check` never builds it in CI.
        sdbisect-revert-vm = import ./nix/sdbisect-revert-vm.nix {
          inherit pkgs;
          revertedSystemd = sdbisectPkgs.systemd;
        };
      };
      lib.withSpec = composixLib.withSpec;
      checks.${system} = {
        compose-fallback-vm = import ./nix/compose-fallback-vm.nix { inherit pkgs cix; };
        vm-dogfood = import ./nix/vm-dogfood.nix { inherit pkgs cix; };
        scenario-lifecycle = import ./nix/scenarios/lifecycle.nix { inherit pkgs cix; };
        scenario-side-by-side = import ./nix/scenarios/side-by-side.nix { inherit pkgs cix; };
        scenario-update-repin = import ./nix/scenarios/update-repin.nix { inherit pkgs cix; };
        scenario-gc-survival = import ./nix/scenarios/gc-survival.nix { inherit pkgs cix; };
        scenario-observability = import ./nix/scenarios/observability.nix { inherit pkgs cix; };
        with-spec-redis = pkgs.runCommand "with-spec-redis-check" { } ''
          test -f ${withSpecRedis}/cix-manifest.json
          test -L ${withSpecRedis}/etc/redis
          test -f ${withSpecRedis}/etc/redis/redis.conf
          touch "$out"
        '';
      };
    };
}
