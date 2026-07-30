{
  description = "composix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.nixpkgs-systemd257.url = "github:NixOS/nixpkgs/0002d4fba62a97fe1260dc41f00deaac9a53f63d";
  # Keep a real 6.17 package available after nixpkgs' EOL aliases turn it into an error.
  inputs.nixpkgs-linux617.url = "github:NixOS/nixpkgs/ef6c19e8baf55f671169995f0fa532511062a99a";

  outputs = { self, nixpkgs, nixpkgs-systemd257, nixpkgs-linux617 }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      systemd257Pkgs = import nixpkgs-systemd257 { inherit system; };
      linux617Pkgs = import nixpkgs-linux617 { inherit system; };
      kernel617Packages = linux617Pkgs.linuxPackages_6_17;
      systemd257Compat = systemd257Pkgs.runCommand "systemd-257.6-nixos-module-compat" { } ''
        cp -a ${systemd257Pkgs.systemd}/. "$out"
        chmod -R u+w "$out"
        old_systemd_path=${systemd257Pkgs.systemd}
        grep -rlIF "$old_systemd_path" "$out" | xargs -r sed -i "s|$old_systemd_path|$out|g"
        mkdir -p "$out/example/systemd/system"
        chmod u+w "$out/example/systemd/system"
        for source_unit in ${pkgs.systemd}/example/systemd/system/*; do
          unit_name=$(basename "$source_unit")
          test -e "$out/example/systemd/system/$unit_name" || printf '%s\n' '[Unit]' 'Description=Compatibility placeholder for newer NixOS module' > "$out/example/systemd/system/$unit_name"
        done
      '';
      systemd257 = systemd257Compat // {
        inherit (systemd257Pkgs.systemd) kbd util-linux;
        interfaceVersion = 2;
        out = systemd257Compat;
        withBootloader = false;
        withCryptsetup = false;
        withEfi = false;
        withFido2 = false;
        withImportd = false;
        withLogind = true;
        withMachined = false;
        withNspawn = false;
        withPortabled = false;
        withRepart = false;
        withSysupdate = false;
        withTpm2Units = false;
        withTimedated = false;
        withLocaled = false;
        withHostnamed = false;
        withUtmp = true;
      };
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
          inherit systemd257;
          inherit kernel617Packages;
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
