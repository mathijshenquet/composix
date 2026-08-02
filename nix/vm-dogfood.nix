{ pkgs, cix }:

let
  postgresTemplate =
    (pkgs.extend (import ../examples/pack/postgres/postgres.nix)).composixPostgresTemplate;
  nginx = pkgs.runCommand "nginx-cix" { } ''
    mkdir -p $out/etc/nginx $out/srv/www
    ln -s ${pkgs.nginx}/conf/mime.types $out/etc/nginx/mime.types
    install -m 0644 ${../examples/pack/nginx/index.html} $out/srv/www/index.html
    install -m 0644 ${../examples/pack/nginx/nginx.conf} $out/etc/nginx/nginx.conf
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 0,
          "start": ["${pkgs.nginx}/bin/nginx", "-c", "/etc/nginx/nginx.conf", "-e", "stderr"],
          "mounts": ["/etc/nginx", "/srv/www"],
          "ports": { "http": { "value": 8080, "protocol": "tcp" } },
          "dirs": {
            "cache": ["/var/cache/nginx"],
            "run": ["/run/nginx"]
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
    cp -R ${postgresTemplate} $out/opt/postgres/template
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 0,
          "start_pre": ["${pkgs.bash}/bin/sh", "/opt/postgres/setup"],
          "start": ["${pkgs.bash}/bin/sh", "/opt/postgres/start", "$PORT"],
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
    EOF
  '';
  redis = import ../examples/pack/redis { inherit pkgs; };
  caddy = pkgs.runCommand "caddy-cix" { } ''
    mkdir -p $out/bin $out/srv/www
    ln -s ${pkgs.caddy}/bin/caddy $out/bin/caddy
    install -m 0644 ${../examples/pack/caddy/index.html} $out/srv/www/index.html
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 0,
          "start": ["bin/caddy", "file-server", "--root", "/srv/www", "--listen", ":80"],
          "mounts": ["/srv/www"],
          "ports": { "http": { "value": 80, "protocol": "tcp" } }
    }
    EOF
  '';
  logsProbe = pkgs.runCommand "logs-probe-cix" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/logs-probe" <<'EOF'
    #!${pkgs.runtimeShell}
    set -eu
    marker=/app/logs/restart-marker
    if test -e "$marker"; then
      test "$( ${pkgs.coreutils}/bin/cat "$marker")" = persists
    else
      printf '%s\n' persists > "$marker"
    fi
    exec ${pkgs.coreutils}/bin/sleep 300
    EOF
    chmod +x "$out/bin/logs-probe"
    cat > "$out/cix-manifest.json" <<'EOF'
    {
      "cixManifest": 0,
      "start": ["bin/logs-probe"],
      "dirs": {"logs": ["/app/logs"]}
    }
    EOF
  '';
  nodeApp = pkgs.runCommand "node-app-cix" { } ''
    mkdir -p $out/bin $out/app
    ln -s ${pkgs.nodejs}/bin/node $out/bin/node
    install -m 0644 ${../examples/pack/node-app/server.js} $out/app/server.js
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 0,
          "start": ["bin/node", "/app/server.js"],
          "mounts": ["/app"],
          "ports": { "http": { "value": 8081, "protocol": "tcp" } },
          "claims": ["jit"]
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
      "cixManifest": 0,
      "start": ["bin/pid-probe"]
    }
    EOF
  '';
  timerApp = pkgs.runCommand "timer-app-cix" { } ''
    mkdir -p "$out/bin"
    ln -s ${pkgs.coreutils}/bin/true "$out/bin/timer-app"
    cat > "$out/cix-manifest.json" <<'EOF'
    {
      "cixManifest": 0,
      "kind": "app",
      "start": ["bin/timer-app"]
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
    logs_item = machine.succeed("nix-store --add ${logsProbe}").strip()
    logs_unit = machine.succeed("cix run " + logs_item + " --detach").strip()
    machine.succeed("systemctl is-active " + logs_unit)
    logs_base = logs_unit.removesuffix(".service").rsplit("-", 1)[0]
    logs_marker = "/var/log/" + logs_base + "/app/logs/restart-marker"
    machine.wait_until_succeeds("test -f " + logs_marker + " && grep -Fx persists " + logs_marker)
    machine.succeed("systemctl show " + logs_unit + " --property=Environment --value | grep -F 'LOGS_DIRECTORY=/app/logs'")
    machine.succeed("systemctl stop " + logs_unit)
    logs_restart = machine.succeed("cix run " + logs_item + " --detach").strip()
    machine.succeed("systemctl is-active " + logs_restart)
    machine.succeed("grep -Fx persists " + logs_marker)
    machine.succeed("systemctl stop " + logs_restart)
    logs_degraded = machine.succeed("CIX_PRIVATE_PIDS_PROBE=unsupported cix run " + logs_item + " --detach").strip()
    machine.succeed("systemctl is-active " + logs_degraded)
    machine.succeed("systemctl show " + logs_degraded + " --property=PrivatePIDs --value | grep -Fx no")
    machine.succeed("stat -c %a " + logs_marker.rsplit("/", 1)[0] + " | grep -Fx 733")
    machine.succeed("grep -Fx persists " + logs_marker)
    machine.succeed("systemctl stop " + logs_degraded)

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

    timer_app = machine.succeed("nix-store --add ${timerApp}").strip()
    timer_unit = machine.succeed("cix run " + timer_app + " --schedule 'Mon *-*-* 12:00:00'").strip()
    timer_root = "/run/cix/gcroots/" + timer_unit + ".root"
    machine.succeed("systemctl is-active " + timer_unit)
    machine.succeed("test -L " + timer_root)
    machine.succeed("systemctl list-timers --all | grep -F " + timer_unit)
    machine.succeed("systemctl stop " + timer_unit)
    # Root removal rides the PartOf-propagated stop of the companion
    # gc-root unit — asynchronous relative to the timer's own stop job.
    machine.wait_until_succeeds("test ! -L " + timer_root)

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
    redis_base = redis_unit.removesuffix(".service").rsplit("-", 1)[0]
    redis_socket = "/run/" + redis_base + "/run/redis/redis.sock"
    machine.wait_until_succeeds("${pkgs.redis}/bin/redis-cli -h 127.0.0.1 -p 6379 PING | grep -Fx PONG")
    machine.wait_until_succeeds("${pkgs.redis}/bin/redis-cli -s " + redis_socket + " PING | grep -Fx PONG")
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

    # Global GC may collect any not-yet-added additionalPaths item (they are
    # valid but unrooted in the VM store image), so it must run after the
    # last nix-store --add. The host-store 9p mode used locally hides this.
    machine.succeed("nix-store --gc --max-freed 1 >/dev/null")
    machine.succeed("test -z \"$(find /nix/var/nix/gcroots/auto -type l -lname " + pid_root + ")\"")
  '';
}
