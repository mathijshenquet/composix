{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  web = pkgs.runCommand "scenario-health-web" { } ''
    mkdir -p "$out/opt/health"
    cat > "$out/opt/health/web.py" <<'PY'
    import os
    import socket
    import time

    state = "/var/lib/health"
    os.makedirs(state, exist_ok=True)
    counter_path = os.path.join(state, "starts")
    try:
        with open(counter_path, "r", encoding="utf-8") as handle:
            starts = int(handle.read()) + 1
    except (FileNotFoundError, ValueError):
        starts = 1
    with open(counter_path, "w", encoding="utf-8") as handle:
        handle.write(str(starts))

    hang = os.path.join(state, "hang")
    if os.path.exists(hang):
        os.unlink(hang)
    print("health-instance {} starting".format(starts), flush=True)
    time.sleep(3)
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", 18090))
    listener.listen()
    listener.settimeout(0.2)
    os.makedirs("/run/health", exist_ok=True)
    with open("/run/health/producer-ready", "w", encoding="utf-8") as handle:
        handle.write("ready\n")
    print("health-instance {} ready".format(starts), flush=True)

    while True:
        if os.path.exists(hang):
            print("health-instance {} hung".format(starts), flush=True)
            while True:
                time.sleep(60)
        try:
            connection, _ = listener.accept()
        except TimeoutError:
            continue
        with connection:
            connection.settimeout(2)
            try:
                connection.recv(4096)
                body = "ok\n"
                response = "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}".format(len(body), body)
                connection.sendall(response.encode("ascii"))
            except OSError:
                pass
    PY
    chmod 0755 "$out/opt/health/web.py"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${pkgs.python3}/bin/python3","opt/health/web.py"],"mounts":["/opt/health"],"ports":{"http":{"value":18090,"protocol":"tcp"}},"dirs":{"state":["/var/lib/health"],"run":["/run/health"]},"readiness":{"type":"http","target":":18090/healthz","timeout":"10s"},"liveness":{"type":"http","target":":18090/livez","interval":"1s"}}
    EOF
  '';

  consumer = pkgs.runCommand "scenario-health-consumer" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/consumer" <<'SH'
    #!${pkgs.python3}/bin/python3
    import os
    import signal
    import sys

    if not os.path.isfile("/run/producer/producer-ready"):
        raise SystemExit("producer readiness marker is absent")
    print("structural-consumer-after-readiness", flush=True)
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    while True:
        signal.pause()
    SH
    chmod 0755 "$out/bin/consumer"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/consumer"]}
    EOF
  '';

  failing = pkgs.runCommand "scenario-health-failing" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/failing" <<'SH'
    #!${pkgs.runtimeShell}
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/failing"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/failing"],"ports":{"http":{"value":18091,"protocol":"tcp"}},"readiness":{"type":"http","target":":18091/healthz","timeout":"2s"}}
    EOF
  '';

  compose = pkgs.writeText "scenario-health.json" ''
    {
      "composeVersion": 1,
      "name": "health",
      "services": {
        "consumer": { "item": "scenario-health-consumer:v1" },
        "web": { "item": "scenario-health-web:v1" }
      },
      "edges": {
        "producer": {
          "producer": { "service": "web", "path": "/run/health" },
          "consumers": { "consumer": { "path": "/run/producer" } }
        }
      }
    }
  '';

  failingCompose = pkgs.writeText "scenario-health-failing.json" ''
    {
      "composeVersion": 1,
      "name": "healthfail",
      "services": {
        "bad": { "item": "scenario-health-failing:v1" }
      }
    }
  '';
in
scenario.node ''
  import time

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${web}) scenario-health-web:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${consumer}) scenario-health-consumer:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${failing}) scenario-health-failing:v1")
  machine.succeed("cp ${compose} /tmp/scenario/health.json; cp ${failingCompose} /tmp/scenario/health-failing.json")

  started = time.time()
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/health.json")
  elapsed = time.time() - started
  assert elapsed >= 2.5, "cix up returned before readiness: {:.2f}s".format(elapsed)
  machine.succeed("systemctl is-active cix-health-web.service cix-health-consumer.service")
  machine.succeed("journalctl --no-pager -u cix-health-consumer.service | grep -F structural-consumer-after-readiness")
  machine.succeed("systemctl show cix-health-consumer.service -p After --value | grep -F cix-health-web.service")
  machine.succeed("systemctl show cix-health-web.service -p WatchdogUSec --value | grep -Fx 3s")
  machine.succeed("systemctl show cix-health-web.service -p Restart --value | grep -Fx on-failure")
  machine.succeed("systemctl cat cix-health-web.service | grep -F 'probe\" \"await\" \"http\" \":18090/healthz'")
  machine.succeed("systemctl cat cix-health-web.service | grep -F 'probe\" \"pinger\" \"http\" \":18090/livez'")
  machine.succeed("! systemctl cat cix-health-web.service | grep -E '(curl|/bin/sh)'")

  status, output = machine.execute("timeout 30 env CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/health-failing.json 2>&1")
  print(output)
  assert status != 0
  assert "failed" in output.lower()
  machine.succeed("systemctl is-failed cix-healthfail-bad.service")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down healthfail")
  machine.succeed("systemctl reset-failed 'cix-healthfail*' || true")

  before = int(machine.succeed("systemctl show cix-health-web.service -p NRestarts --value").strip())
  machine.succeed("touch /var/lib/cix-health-web/var/lib/health/hang")
  machine.wait_until_succeeds("test $(systemctl show cix-health-web.service -p NRestarts --value) -gt " + str(before) + " && systemctl is-active cix-health-web.service", timeout=30)
  machine.wait_until_succeeds("journalctl --no-pager -u cix-health-web.service | grep -F 'liveness watchdog missed'", timeout=10)
  machine.wait_until_succeeds("curl --max-time 2 --fail --silent http://127.0.0.1:18090/healthz | grep -Fx ok", timeout=20)
  machine.succeed("test $(cat /var/lib/cix-health-web/var/lib/health/starts) -ge 2")

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down health")
  machine.succeed("systemctl reset-failed 'cix-health*' || true")
  machine.succeed("test -z \"$(systemctl list-units --all --no-legend 'cix-health*' | awk 'NF { print $1 }')\"")
''
