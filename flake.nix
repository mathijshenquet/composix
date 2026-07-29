{
  description = "composix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      cix = pkgs.rustPlatform.buildRustPackage {
        pname = "cix";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "-p" "cix" ];
        doCheck = false;
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
      };
      lib.withSpec = composixLib.withSpec;
      checks.${system} = {
        vm-dogfood = import ./nix/vm-dogfood.nix { inherit pkgs cix; };
        with-spec-redis = pkgs.runCommand "with-spec-redis-check" { } ''
          test -f ${withSpecRedis}/cix-manifest.json
          test -L ${withSpecRedis}/etc/redis
          test -f ${withSpecRedis}/etc/redis/redis.conf
          touch "$out"
        '';
      };
    };
}
