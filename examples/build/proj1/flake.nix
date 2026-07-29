{
  description = "Generic multi-output Rust and frontend build shape";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
      inherit (pkgs) lib;

      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
      members = (builtins.fromTOML (builtins.readFile ./rust/Cargo.toml)).workspace.members;

      mkBinSrc =
        name: keepCrates:
        lib.cleanSourceWith {
          src = ./rust;
          name = "rust-${name}-source";
          filter =
            path: type:
            let
              rel = lib.removePrefix (toString ./rust + "/") (toString path);
              parts = lib.splitString "/" rel;
              base = baseNameOf (toString path);
              topDir = lib.head parts;
              ignored =
                type == "directory"
                && lib.elem base [
                  "target"
                  ".git"
                  ".devenv"
                ];
              manifest = lib.elem base [
                "Cargo.toml"
                "Cargo.lock"
                "rust-toolchain.toml"
              ];
              inKept = lib.elem topDir keepCrates;
              keptSource = inKept && lib.hasSuffix ".rs" rel;
              memberEntrypoint = lib.elem topDir members && !inKept && rel == "${topDir}/src/main.rs";
            in
            !ignored && (type == "directory" || manifest || keptSource || memberEntrypoint);
        };

      rustSource = lib.cleanSourceWith {
        src = ./rust;
        name = "rust-workspace-source";
        filter =
          path: type:
          let
            base = baseNameOf (toString path);
          in
          (
            type == "directory"
            && !lib.elem base [
              "target"
              ".git"
              ".devenv"
            ]
          )
          || lib.hasSuffix ".rs" path
          || lib.hasSuffix ".toml" path
          || base == "Cargo.lock";
      };

      commonArgs = {
        src = rustSource;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly (
        commonArgs
        // {
          pname = "rust-workspace-deps";
          version = "0.1.0";
        }
      );

      mkBin =
        { name, keepCrates }:
        craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = name;
            version = "0.1.0";
            src = mkBinSrc name keepCrates;
            cargoExtraArgs = "--bin ${name}";
            doCheck = false;
          }
        );

      api = mkBin {
        name = "api";
        keepCrates = [
          "common"
          "api"
        ];
      };
      worker = mkBin {
        name = "worker";
        keepCrates = [
          "common"
          "worker"
        ];
      };
      dashboard = mkBin {
        name = "dashboard";
        keepCrates = [
          "common"
          "dashboard"
        ];
      };

      frontendSource = lib.cleanSourceWith {
        src = ./frontend;
        name = "frontend-source";
        filter =
          path: type:
          let
            base = baseNameOf (toString path);
          in
          !(
            type == "directory"
            && lib.elem base [
              "node_modules"
              "dist"
            ]
          );
      };
      frontendDeps = pkgs.fetchPnpmDeps {
        pname = "frontend";
        version = "0.1.0";
        src = frontendSource;
        fetcherVersion = 4;
        hash = "sha256-dIp6CNh1Kn4aqJWku1G/FUdn/u+epzhqlqwnAkB2uW0=";
      };
      frontend = pkgs.stdenvNoCC.mkDerivation {
        pname = "frontend";
        version = "0.1.0";
        src = frontendSource;
        pnpmDeps = frontendDeps;
        nativeBuildInputs = [
          pkgs.nodejs
          pkgs.pnpm
          pkgs.pnpmConfigHook
        ];
        buildPhase = ''
          runHook preBuild
          pnpm build
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          cp -r dist "$out"
          runHook postInstall
        '';
      };
    in
    {
      packages.${system} = {
        inherit
          api
          worker
          dashboard
          frontend
          ;
        default = api;
      };
      checks.${system} = {
        inherit
          api
          worker
          dashboard
          frontend
          ;
      };
      devShells.${system}.default = craneLib.devShell {
        checks = self.checks.${system};
        packages = [
          pkgs.nodejs
          pkgs.pnpm
        ];
      };
      formatter.${system} = pkgs.nixfmt-tree;
    };
}
