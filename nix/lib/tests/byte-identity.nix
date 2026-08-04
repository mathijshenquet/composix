{
  pkgs,
  cix,
  project,
  nixpkgsSource,
}:

let
  offlineNix = pkgs.writeShellScriptBin "nix" ''
    exec ${pkgs.nix}/bin/nix --offline "$@"
  '';
  offlinePath = "${offlineNix}/bin:/run/current-system/sw/bin";
  offlineCix = pkgs.writeShellScriptBin "cix" ''
    export XDG_CACHE_HOME=/root/.cache
    export PATH=${offlineNix}/bin:${pkgs.lib.makeBinPath [ pkgs.bubblewrap pkgs.strace ]}:$PATH
    exec ${cix}/bin/.cix-wrapped "$@"
  '';
  fetcherCache = pkgs.runCommand "cip94-fetcher-cache" {
    nativeBuildInputs = [ pkgs.git pkgs.sqlite ];
  } ''
    mkdir -p "$out/nix"
    git init --bare --quiet "$out/nix/tarball-cache-v2"
    git --git-dir="$out/nix/tarball-cache-v2" --work-tree=${nixpkgsSource} add -f .
    tree=$(git --git-dir="$out/nix/tarball-cache-v2" write-tree)
    test "$tree" = 75c877e125eee86e5eb69e8f4543f376212a85b8
    git --git-dir="$out/nix/tarball-cache-v2" update-ref refs/tags/cip94-cache "$tree"
    rm "$out/nix/tarball-cache-v2/index"
    git -c pack.threads="$NIX_BUILD_CORES" --git-dir="$out/nix/tarball-cache-v2" gc --prune=now
    sqlite3 "$out/nix/fetcher-cache-v4.sqlite" <<'SQL'
    CREATE TABLE Cache (domain text not null, key text not null, value text not null, timestamp integer not null, primary key(domain,key));
    INSERT INTO Cache VALUES ('gitRevToLastModified', '{"rev":"624af665418d3c65d544145b4d34ad696439570e"}', '{"lastModified":1785090369}', 0);
    INSERT INTO Cache VALUES ('gitRevToTreeHash', '{"rev":"624af665418d3c65d544145b4d34ad696439570e"}', '{"treeHash":"75c877e125eee86e5eb69e8f4543f376212a85b8"}', 0);
    INSERT INTO Cache VALUES ('sourcePathToHash', '{"fingerprint":"624af665418d3c65d544145b4d34ad696439570e","method":"nar","path":"/"}', '{"hash":"sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4="}', 0);
    SQL
  '';
  cixfileLib = import ../default.nix;
  assemblyBuild = cixfileLib.buildCixfile {
    src = ./fixtures/assembly;
    item = "assembly";
  };
  builderBuild = cixfileLib.buildCixfile {
    src = ./fixtures/builder;
    item = "builder";
  };
in
pkgs.testers.runNixOSTest {
  name = "build-cixfile-byte-identity";

  nodes.machine = { ... }: {
    environment.systemPackages = [ cix pkgs.jq pkgs.nix pkgs.sqlite ];
    environment.etc."cip94-project".source = project;
    environment.etc."cip94-nixpkgs".source = nixpkgsSource;
    nix.settings.experimental-features = [ "nix-command" "flakes" ];
    system.extraDependencies = builderBuild.cip94FetchInputDerivations ++ [
      assemblyBuild
      pkgs.bash
      pkgs.bubblewrap
      pkgs.coreutils
      pkgs.patchelf
      pkgs.proot
      pkgs.stdenvNoCC
    ];
    virtualisation.cores = 4;
    virtualisation.memorySize = 3072;
    system.stateVersion = "24.11";
  };

  testScript = ''
    start_all()
    machine.succeed("mkdir -p /var/lib/cix-index /var/lib/cix-workspaces /tmp/cip94 /root/.cache")
    machine.succeed("cp -a ${fetcherCache}/nix /root/.cache/nix")
    machine.succeed("sqlite3 /root/.cache/nix/fetcher-cache-v4.sqlite 'UPDATE Cache SET timestamp = unixepoch();'")

    def fixture(name):
        path = "/tmp/cip94/" + name
        machine.succeed("cp -a ${project}/nix/lib/tests/fixtures/" + name + " " + path)
        machine.succeed("chmod -R u+w " + path)
        return path

    def cold(path, item):
        command = "env CIX_STATE_DIR=/var/lib/cix-index CIX_BUILD_WORKSPACE_DIR=/var/lib/cix-workspaces ${offlineCix}/bin/cix build --cold " + path
        return machine.succeed(command + " | jq -r ." + item).strip()

    def assert_same_nar(left, right):
        left_hash = machine.succeed("nix hash path --mode nar " + left).strip()
        right_hash = machine.succeed("nix hash path --mode nar " + right).strip()
        assert left_hash == right_hash, left + " (" + left_hash + ") != " + right + " (" + right_hash + ")"

    assembly = fixture("assembly")
    assert_same_nar(cold(assembly, "assembly"), "${assemblyBuild}")

    builder = fixture("builder")
    machine.succeed("env CIX_STATE_DIR=/var/lib/cix-index CIX_BUILD_WORKSPACE_DIR=/var/lib/cix-workspaces ${offlineCix}/bin/cix build --update-lock=build " + builder)
    cold_builder = cold(builder, "builder")
    machine.succeed("sysctl -w user.max_user_namespaces=0")
    machine.succeed("test $(sysctl -n user.max_user_namespaces) = 0")
    machine.fail("runuser -u nixbld1 -- unshare --user true")
    eval_builder = machine.succeed("env PATH=${offlinePath} nix build --impure --print-out-paths --no-link --expr '(import ${project}/nix/lib/default.nix).buildCixfile { src = " + builder + "; item = \"builder\"; system = \"x86_64-linux\"; }'").strip()
    assert_same_nar(cold_builder, eval_builder)

    fhs = fixture("fhs-boundary")
    status, output = machine.execute("env PATH=${offlinePath} nix build --impure --no-link --expr '(import ${project}/nix/lib/default.nix).buildCixfile { src = " + fhs + "; item = \"fhs-boundary\"; system = \"x86_64-linux\"; }' 2>&1")
    assert status != 0
    assert "CIP-95 FHS loader surface" in output, output
  '';
}
