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
    os.chmod(path, 0o666)
    listener.listen()
    print("db-line ready", flush=True)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.settimeout(2)
            try:
                connection.recv(1024)
                connection.sendall(b"PONG")
            except OSError:
                pass
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
        handle.write("survives\n")

    listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
    print("api-line " + os.environ["MESSAGE"], flush=True)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.settimeout(5)
            try:
                connection.recv(4096)
                with socket.socket(socket.AF_UNIX) as db:
                    db.settimeout(2)
                    db.connect("/run/db/db.sock")
                    db.sendall(b"PING")
                    pong = db.recv(1024).decode("ascii")
                if pong != "PONG":
                    raise OSError("unexpected database response")
            except (OSError, UnicodeDecodeError) as error:
                print("api-db failure: " + str(error), flush=True)
                body = "db unavailable\n"
                response = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}".format(len(body), body)
            else:
                body = os.environ["MESSAGE"] + ":" + pong + "\n"
                response = "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}".format(len(body), body)
            try:
                connection.sendall(response.encode("ascii"))
            except OSError:
                pass
    PY
    chmod 0755 "$out/opt/scenario/api.py"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":3,"services":{"api":{"exec":["${python}/bin/python3","opt/scenario/api.py"],"mounts":["/opt/scenario"],"env":{"MESSAGE":{"default":"${message}"}},"listeners":{"http":{"type":"stream"}},"dirs":{"state":["/var/lib/api"],"run":["/run/api"]}}}}
    EOF
  '';

  composeWithUpdate = name: bind: apiRef: message: update:
    let
      env = if message == null then "" else '', "env": { "MESSAGE": "${message}" }'';
      updateField = if update == null then "" else '', "update": "${update}"'';
    in ''
    {
      "composeVersion": 1,
      "name": "${name}",
      "services": {
        "api": { "item": "${apiRef}", "bind": { "http": "${bind}" }${env}${updateField} },
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

  compose = name: bind: apiRef: message:
    composeWithUpdate name bind apiRef message null;

  composeFile = name: bind: apiRef: message:
    pkgs.writeText "scenario-${name}.json" (compose name bind apiRef message);

  trackedComposeFile = name: bind: apiRef: message:
    pkgs.writeText "scenario-${name}.json" (composeWithUpdate name bind apiRef message "track");

  node = script: pkgs.testers.runNixOSTest {
    name = "scenario";
    nodes.machine = { ... }: {
      environment.systemPackages = [ cix pkgs.curl pkgs.jq pkgs.procps pkgs.systemd ];
      environment.variables.NIX_CONFIG = "experimental-features = nix-command flakes";
      networking.useDHCP = false;
      networking.interfaces.eth0.useDHCP = false;
      networking.firewall.enable = false;
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
  inherit api compose composeFile db node trackedComposeFile;
}
