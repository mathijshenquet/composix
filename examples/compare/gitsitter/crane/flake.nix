{
  description = "gitsitter built with crane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/9cf7092bdd603554bd8b63c216e8943cf9b12512";
    crane.url = "github:ipetkov/crane";
    gitsitter = {
      url = "github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd";
      flake = false;
    };
  };

  outputs = { nixpkgs, crane, gitsitter, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      craneLib = crane.mkLib pkgs;
      commonArgs = {
        src = gitsitter;
        strictDeps = true;
        GIT_COMMIT_HASH = "29c8a2d";
        nativeBuildInputs = [ pkgs.pkg-config pkgs.git ];
        buildInputs = [ pkgs.openssl pkgs.libgit2 pkgs.sqlite ];
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    {
      packages.${system}.default = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });
    };
}
