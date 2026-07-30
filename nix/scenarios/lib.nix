{ pkgs, cix }:

let
  python = pkgs.python3;
  db = pkgs.runCommand "scenario-db" { } ''
    mkdir -p "$out/opt/scenario"
    cat > "$out/opt/scenario/db.py" <<'PY'
    import os
    import socket

    path = "/run/db/db.sock"
    os.makedirs(os.path.dirname(path), exist_ok=True)
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass
    listener = socket.socket(socket.AF_UNIX)
    listener.bind(path)
    listener.listen()
    print("db-line ready", flush=True)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(1024)
            connection.sendall(b"PONG")
    PY
    chmod 0755 "$out/opt/scenario/db.py"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":3,"services":{"db":{"exec":["${python}/bin/python3","opt/scenario/db.py"],"mounts":["/opt/scenario"],"dirs":{"run":["/run/db"]}}}}
    EOF
  '';

  api = message: pkgs.runCommand "scenario-api-${message}" { } ''
    mkdir -p "$out/opt/scenario"
    cat > "$out/opt/scenario/api.py" <<'PY'
    import os
    import socket

    state = "/var/lib/api/sentinel"
    os.makedirs(os.path.dirname(state), exist_ok=True)
    with open(state, "w", encoding="utf-8") as handle:
        handle.write("survives\\n")

    listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
    print("api-line " + os.environ["MESSAGE"], flush=True)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(4096)
            db = socket.socket(socket.AF_UNIX)
            db.connect("/run/db/db.sock")
            db.sendall(b"PING")
            pong = db.recv(1024).decode("ascii")
            db.close()
            body = os.environ["MESSAGE"] + ":" + pong + "\\n"
            response = "HTTP/1.1 200 OK\\r\\nContent-Length: {}\\r\\nConnection: close\\r\\n\\r\\n{}".format(len(body), body)
            connection.sendall(response.encode("ascii"))
    PY
    chmod 0755 "$out/opt/scenario/api.py"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":3,"services":{"api":{"exec":["${python}/bin/python3","opt/scenario/api.py"],"mounts":["/opt/scenario"],"env":{"MESSAGE":{"default":"${message}"}},"listeners":{"http":{"type":"stream"}},"dirs":{"state":["/var/lib/api"],"run":["/run/api"]}}}}
    EOF
  '';

  compose = name: bind: apiRef: message:
    let env = if message == null then "" else '', "env": { "MESSAGE": "${message}" }''; in ''
    {
      "composeVersion": 1,
      "name": "${name}",
      "services": {
        "api": { "item": "${apiRef}", "bind": { "http": "${bind}" }${env} },
        "db": { "item": "scenario-db:v1" }
      },
      "edges": {
        "database": {
          "producer": { "service": "db", "path": "/run/db" },
          "consumers": { "api": {} }
        }
      }
    }
  '';

  composeFile = name: bind: apiRef: message:
    pkgs.writeText "scenario-${name}.json" (compose name bind apiRef message);

  node = script: pkgs.testers.runNixOSTest {
    name = "scenario";
    nodes.machine = { ... }: {
      environment.systemPackages = [ cix pkgs.curl pkgs.jq pkgs.procps pkgs.systemd ];
      environment.variables.NIX_CONFIG = "experimental-features = nix-command flakes";
      networking.useDHCP = false;
      networking.interfaces.eth0.useDHCP = false;
      networking.firewall.enable = false;
      boot.kernel.sysctl."kernel.unprivileged_userns_clone" = 1;
      system.stateVersion = "24.11";
    };
    testScript = ''
      start_all()
      machine.succeed("mkdir -p /var/lib/cix-index /tmp/scenario")
      machine.succeed("mkdir /run/cix-units; cp -a /etc/systemd/system/. /run/cix-units/; mount --bind /run/cix-units /etc/systemd/system; systemctl daemon-reload")
      ${script}
    '';
  };
in
{
  inherit api compose composeFile db node;
}
