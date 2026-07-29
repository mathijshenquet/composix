{ pkgs, ... }:

{
  languages.rust.enable = true;

  packages = [
    pkgs.bubblewrap
    pkgs.cargo-watch
  ];
}
