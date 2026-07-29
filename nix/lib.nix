{ pkgs }:

{
  withSpec =
    {
      manifest,
      mounts ? { },
      name ? "cix-item",
    }:
    let
      manifestFile = pkgs.writeText "${name}-cix-manifest.json" (builtins.toJSON manifest + "\n");
      mountCommands = pkgs.lib.mapAttrsToList (
        destination: source: ''
          mkdir -p "$out$(dirname ${builtins.toString destination})"
          ln -s ${source} "$out${destination}"
        ''
      ) mounts;
    in
    pkgs.runCommand name { } ''
      install -Dm0644 ${manifestFile} "$out/cix-manifest.json"
      ${pkgs.lib.concatStringsSep "\n" mountCommands}
    '';
}
