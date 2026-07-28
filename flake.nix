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
    in
    {
      packages.${system}.cix = cix;
      checks.${system}.vm-dogfood = import ./nix/vm-dogfood.nix { inherit pkgs cix; };
    };
}
