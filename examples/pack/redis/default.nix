{
  pkgs ? import <nixpkgs> { },
  composix ? import ../../../nix/lib.nix { inherit pkgs; },
}:

composix.withSpec {
  name = "redis-cix";
  manifest = {
    cixManifest = 0;
    start = [ "${pkgs.redis}/bin/redis-server" "/etc/redis/redis.conf" ];
    mounts = [ "/etc/redis" ];
    env = {
      LANG.default = "C";
      LC_ALL.default = "C";
    };
    ports.redis = {
      value = 6379;
      protocol = "tcp";
    };
    dirs.run = [ "/run/redis" ];
  };
  mounts."/etc/redis" = pkgs.runCommand "redis-config" { } ''
    install -Dm0644 ${./redis.conf} "$out/redis.conf"
  '';
}
