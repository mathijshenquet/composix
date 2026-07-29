{ pkgs, cix }:

let
  nginx = import ../examples/nginx { inherit pkgs; };
  postgres = import ../examples/postgres { inherit pkgs; };
  redis = pkgs.runCommand "redis-cix" { } ''
    mkdir -p $out/bin $out/etc/redis
    ln -s ${pkgs.redis}/bin/redis-cli $out/bin/redis-cli
    ln -s ${pkgs.redis}/bin/redis-server $out/bin/redis-server
    install -m 0644 ${../examples/redis/redis.conf} $out/etc/redis/redis.conf
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "redis": {
          "exec": ["bin/redis-server", "/etc/redis/redis.conf"],
          "mounts": ["/etc/redis"],
          "ports": { "redis": { "value": 6379, "protocol": "tcp" } },
          "dirs": { "run": ["/run/redis"] }
        }
      }
    }
    EOF
  '';
  caddy = pkgs.runCommand "caddy-cix" { } ''
    mkdir -p $out/bin $out/srv/www
    ln -s ${pkgs.caddy}/bin/caddy $out/bin/caddy
    install -m 0644 ${../examples/caddy/index.html} $out/srv/www/index.html
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "caddy": {
          "exec": ["bin/caddy", "file-server", "--root", "/srv/www", "--listen", ":80"],
          "mounts": ["/srv/www"],
          "ports": { "http": { "value": 80, "protocol": "tcp" } }
        }
      }
    }
    EOF
  '';
  nodeApp = pkgs.runCommand "node-app-cix" { } ''
    mkdir -p $out/bin $out/app
    ln -s ${pkgs.nodejs}/bin/node $out/bin/node
    install -m 0644 ${../examples/node-app/server.js} $out/app/server.js
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "node-app": {
          "exec": ["bin/node", "/app/server.js"],
          "mounts": ["/app"],
          "ports": { "http": { "value": 8081, "protocol": "tcp" } },
          "jit": true
        }
      }
    }
    EOF
  '';
in
pkgs.testers.runNixOSTest {
  name = "vm-dogfood";

  nodes.machine = { ... }: {
    environment.systemPackages = [ cix pkgs.curl ];

    networking.useDHCP = false;
    networking.interfaces.eth0.useDHCP = false;
    networking.firewall.enable = false;
    system.stateVersion = "24.11";
  };

  testScript = ''
    start_all()

    nginx_unit = machine.succeed("cix run ${nginx} --detach").strip()
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:8080/ | grep -F 'hello from composix'")
    machine.succeed("cix ps | grep -F " + nginx_unit)
    machine.succeed("systemctl stop " + nginx_unit)

    postgres_unit = machine.succeed("cix run ${postgres} --detach").strip()
    machine.wait_until_succeeds(
        "${postgres}/bin/psql --host=127.0.0.1 --port=5432 --username=cix --dbname=postgres --no-password --tuples-only --no-align --command='SELECT 1' | grep -Fx 1"
    )
    machine.succeed("cix ps | grep -F " + postgres_unit)
    machine.succeed("systemctl stop " + postgres_unit)

    redis_unit = machine.succeed("cix run ${redis} --detach").strip()
    machine.wait_until_succeeds("${redis}/bin/redis-cli -h 127.0.0.1 -p 6379 PING | grep -Fx PONG")
    machine.wait_until_succeeds("${redis}/bin/redis-cli -s /run/redis/redis.sock PING | grep -Fx PONG")
    machine.succeed("cix ps | grep -F " + redis_unit)
    machine.succeed("systemctl stop " + redis_unit)

    caddy_unit = machine.succeed("cix run ${caddy} --detach").strip()
    machine.succeed("systemctl show " + caddy_unit + " --property=AmbientCapabilities --value | grep -Fx cap_net_bind_service")
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:80/ | grep -F 'hello from composix caddy'")
    machine.succeed("cix ps | grep -F " + caddy_unit)
    machine.succeed("systemctl stop " + caddy_unit)

    node_unit = machine.succeed("cix run ${nodeApp} --detach").strip()
    machine.succeed("systemctl show " + node_unit + " --property=MemoryDenyWriteExecute --value | grep -Fx no")
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:8081/ | grep -Fx 'node JIT is enabled'")
    machine.succeed("cix ps | grep -F " + node_unit)
    machine.succeed("systemctl stop " + node_unit)

    machine.succeed("systemctl stop cix-run.slice")
    machine.succeed("test -z \"$(systemctl list-units --no-legend 'cix-*' | awk 'NF { print $1 }')\"")
  '';
}
