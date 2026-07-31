{ pkgs, cix }:

let
  nginx = pkgs.runCommand "nginx-cix" { } ''
    mkdir -p $out/etc/nginx $out/srv/www
    ln -s ${pkgs.nginx}/conf/mime.types $out/etc/nginx/mime.types
    install -m 0644 ${../examples/pack/nginx/index.html} $out/srv/www/index.html
    install -m 0644 ${../examples/pack/nginx/nginx.conf} $out/etc/nginx/nginx.conf
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "nginx": {
          "exec": ["${pkgs.nginx}/bin/nginx", "-c", "/etc/nginx/nginx.conf", "-e", "stderr"],
          "mounts": ["/etc/nginx", "/srv/www"],
          "ports": { "http": { "value": 8080, "protocol": "tcp" } },
          "dirs": {
            "cache": ["/var/cache/nginx"],
            "run": ["/run/nginx"]
          }
        }
      }
    }
    EOF
  '';
  postgres = pkgs.runCommand "postgres-cix" { } ''
    mkdir -p $out/bin $out/opt/postgres
    ln -s ${pkgs.nss_wrapper}/lib/libnss_wrapper.so $out/opt/postgres/libnss_wrapper.so
    ln -s ${pkgs.postgresql}/bin/psql $out/bin/psql
    install -m 0644 ${../examples/pack/postgres/runtime-env.sh} $out/opt/postgres/runtime-env.sh
    install -m 0755 ${../examples/pack/postgres/setup} $out/opt/postgres/setup
    install -m 0755 ${../examples/pack/postgres/start} $out/opt/postgres/start
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "postgres": {
          "setup": ["${pkgs.bash}/bin/sh", "/opt/postgres/setup"],
          "exec": ["${pkgs.bash}/bin/sh", "/opt/postgres/start", "$PORT"],
          "env": {
            "PATH": { "default": "${pkgs.postgresql}/bin:${pkgs.coreutils}/bin" },
            "PORT": { "default": "5432" }
          },
          "mounts": ["/opt/postgres"],
          "ports": { "postgres": { "env": "PORT", "protocol": "tcp" } },
          "dirs": {
            "state": ["/var/lib/postgresql"],
            "run": ["/run/postgresql"]
          }
        }
      }
    }
    EOF
  '';
  redis = import ../examples/pack/redis { inherit pkgs; };
  caddy = pkgs.runCommand "caddy-cix" { } ''
    mkdir -p $out/bin $out/srv/www
    ln -s ${pkgs.caddy}/bin/caddy $out/bin/caddy
    install -m 0644 ${../examples/pack/caddy/index.html} $out/srv/www/index.html
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
    install -m 0644 ${../examples/pack/node-app/server.js} $out/app/server.js
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
  pidProbe = pkgs.runCommand "pid-probe-cix" { } ''
    mkdir -p $out/bin
    cat > $out/bin/pid-probe <<'EOF'
    #!${pkgs.runtimeShell}
    trap 'exit 0' TERM INT
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
    EOF
    chmod +x $out/bin/pid-probe
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 2,
      "services": {
        "pid-probe": { "exec": ["bin/pid-probe"] }
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
    pid_probe = machine.succeed("nix-store --add ${pidProbe}").strip()
    pid_unit = machine.succeed("cix run " + pid_probe + " --detach").strip()
    pid_root = "/run/cix/gcroots/" + pid_unit + ".root"
    machine.succeed("test -L " + pid_root)
    machine.succeed("readlink " + pid_root + " | grep -Fx " + pid_probe)
    machine.succeed("systemctl show " + pid_unit + " --property=ExecStopPost --value | grep -F " + pid_root)
    machine.succeed("find /nix/var/nix/gcroots/auto -type l -lname " + pid_root + " | grep -q .")
    machine.succeed("systemctl show " + pid_unit + " --property=PrivatePIDs --value | grep -Fx yes")
    machine.succeed("cix exec " + pid_unit + " --root -- /bin/sh -c 'read -r comm < /proc/1/comm; test \"$comm\" = pid-probe'")
    machine.succeed("systemctl stop " + pid_unit)
    machine.succeed("test ! -L " + pid_root)
    machine.succeed("nix-store --gc --max-freed 1 >/dev/null")
    machine.succeed("test -z \"$(find /nix/var/nix/gcroots/auto -type l -lname " + pid_root + ")\"")

    nginx_item = machine.succeed("nix-store --add ${nginx}").strip()
    nginx_unit = machine.succeed("cix run " + nginx_item + " --detach").strip()
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:8080/ | grep -F 'hello from composix'")
    machine.succeed("cix ps | grep -F " + nginx_unit)
    machine.succeed("systemctl stop " + nginx_unit)

    postgres_item = machine.succeed("nix-store --add ${postgres}").strip()
    postgres_unit = machine.succeed("cix run " + postgres_item + " --detach").strip()
    machine.wait_until_succeeds(
        "${postgres}/bin/psql --host=127.0.0.1 --port=5432 --username=cix --dbname=postgres --no-password --tuples-only --no-align --command='SELECT 1' | grep -Fx 1"
    )
    machine.succeed("cix ps | grep -F " + postgres_unit)
    machine.succeed("systemctl stop " + postgres_unit)

    redis_item = machine.succeed("nix-store --add ${redis}").strip()
    redis_unit = machine.succeed("cix run " + redis_item + " --detach").strip()
    machine.wait_until_succeeds("${pkgs.redis}/bin/redis-cli -h 127.0.0.1 -p 6379 PING | grep -Fx PONG")
    machine.wait_until_succeeds("${pkgs.redis}/bin/redis-cli -s /run/redis/redis.sock PING | grep -Fx PONG")
    machine.succeed("cix ps | grep -F " + redis_unit)
    machine.succeed("systemctl stop " + redis_unit)

    caddy_item = machine.succeed("nix-store --add ${caddy}").strip()
    caddy_unit = machine.succeed("cix run " + caddy_item + " --detach").strip()
    machine.succeed("systemctl show " + caddy_unit + " --property=AmbientCapabilities --value | grep -Fx cap_net_bind_service")
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:80/ | grep -F 'hello from composix caddy'")
    machine.succeed("cix ps | grep -F " + caddy_unit)
    machine.succeed("systemctl stop " + caddy_unit)

    node_item = machine.succeed("nix-store --add ${nodeApp}").strip()
    node_unit = machine.succeed("cix run " + node_item + " --detach").strip()
    machine.succeed("systemctl show " + node_unit + " --property=MemoryDenyWriteExecute --value | grep -Fx no")
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:8081/ | grep -Fx 'node JIT is enabled'")
    machine.succeed("cix ps | grep -F " + node_unit)
    machine.succeed("systemctl stop " + node_unit)

    machine.succeed("systemctl stop cix-run.slice")
    machine.succeed("test -z \"$(systemctl list-units --no-legend 'cix-*' | awk 'NF { print $1 }')\"")
  '';
}
