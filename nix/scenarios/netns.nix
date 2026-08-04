{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  python = pkgs.python3;
  fixedServer = pkgs.runCommand "scenario-netns-fixed" { } ''
    mkdir -p "$out/opt/netns"
    cat > "$out/opt/netns/fixed.py" <<'PY'
    import socket

    listener = socket.socket()
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", 8080))
    listener.listen()
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(4096)
            body = b"fixed-pod\n"
            connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\n" + body)
    PY
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${python}/bin/python3","opt/netns/fixed.py"],"mounts":["/opt/netns"],"ports":{"http":{"value":8080,"protocol":"tcp"}}}
    EOF
  '';
  fdServer = pkgs.runCommand "scenario-netns-fd" { } ''
    mkdir -p "$out/opt/netns"
    cat > "$out/opt/netns/fd.py" <<'PY'
    import os
    import socket

    listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(4096)
            body = (os.environ["MESSAGE"] + "\n").encode()
            header = f"HTTP/1.1 200 OK\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode()
            connection.sendall(header + body)
    PY
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${python}/bin/python3","opt/netns/fd.py"],"mounts":["/opt/netns"],"env":{"MESSAGE":{"default":"default"}},"listeners":{"http":{"type":"stream"}}}
    EOF
  '';
  egressProbe = pkgs.runCommand "scenario-netns-egress" { } ''
    mkdir -p "$out/opt/netns"
    cat > "$out/opt/netns/egress.py" <<'PY'
    import os
    import socket
    import time

    result = "denied"
    for _ in range(50):
        try:
            address = socket.gethostbyname("egress.test")
            with socket.create_connection((address, 19090), timeout=0.2):
                result = "allowed"
                break
        except OSError:
            time.sleep(0.1)
    os.makedirs("/run/netns-probe", exist_ok=True)
    with open("/run/netns-probe/result", "w", encoding="utf-8") as handle:
        handle.write(result + "\n")
    time.sleep(3600)
    PY
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${python}/bin/python3","opt/netns/egress.py"],"mounts":["/opt/netns"],"dirs":{"run":["/run/netns-probe"]},"claims":["egress"]}
    EOF
  '';
  dnsServer = pkgs.writeText "scenario-netns-dns.py" ''
    import socket

    server = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server.setsockopt(socket.SOL_IP, 15, 1)
    server.bind(("10.231.0.1", 53))
    while True:
        request, peer = server.recvfrom(4096)
        offset = 12
        while request[offset] != 0:
            offset += request[offset] + 1
        question_end = offset + 5
        response = (
            request[:2]
            + b"\x81\x80"
            + request[4:6]
            + b"\x00\x01\x00\x00\x00\x00"
            + request[12:question_end]
            + b"\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x3c\x00\x04"
            + socket.inet_aton("10.231.0.1")
        )
        server.sendto(response, peer)
  '';
  edgeProducer = pkgs.runCommand "scenario-netns-edge-producer" { } ''
    mkdir -p "$out/opt/netns"
    cat > "$out/opt/netns/producer.py" <<'PY'
    import os
    import socket

    path = "/run/netns-edge/edge.sock"
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass
    listener = socket.socket(socket.AF_UNIX)
    listener.bind(path)
    listener.listen()
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(1024)
            connection.sendall(b"edge-ok\n")
    PY
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${python}/bin/python3","opt/netns/producer.py"],"mounts":["/opt/netns"],"dirs":{"run":["/run/netns-edge"]}}
    EOF
  '';
  edgeConsumer = pkgs.runCommand "scenario-netns-edge-consumer" { } ''
    mkdir -p "$out/opt/netns"
    cat > "$out/opt/netns/consumer.py" <<'PY'
    import socket

    listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
    while True:
        connection, _ = listener.accept()
        with connection:
            connection.recv(4096)
            with socket.socket(socket.AF_UNIX) as edge:
                edge.connect("/run/netns-edge/edge.sock")
                edge.sendall(b"hello")
                body = edge.recv(1024)
            header = f"HTTP/1.1 200 OK\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode()
            connection.sendall(header + body)
    PY
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["${python}/bin/python3","opt/netns/consumer.py"],"mounts":["/opt/netns"],"listeners":{"http":{"type":"stream"}},"dirs":{"run":["/run/netns-edge"]}}
    EOF
  '';
  root = pkgs.writeText "scenario-netns.json" ''
    {
      "cixCompose": 1,
      "name": "netns",
      "children": {
        "edgehost": {"item": "scenario-netns-edge-producer:v1"},
        "a": {
          "network": "pod",
          "children": {
            "fixed": {"item": "scenario-netns-fixed:v1", "bind": {"http": "127.0.0.1:18081"}},
            "fd": {"item": "scenario-netns-fd:v1", "env": {"MESSAGE": "v1"}, "bind": {"http": "127.0.0.1:18080"}},
            "allowed": {"item": "scenario-netns-egress:v1"},
            "denied": {"item": "scenario-netns-egress:v1", "egress": false},
            "edgeclient": {"item": "scenario-netns-edge-consumer:v1", "bind": {"http": "127.0.0.1:18082"}}
          },
          "publish": {
            "fixed": {"child": "fixed", "port": "http"},
            "fd": {"child": "fd", "port": "http"},
            "edge": {"child": "edgeclient", "port": "http"}
          }
        },
        "b": {
          "network": "pod",
          "children": {"fixed": {"item": "scenario-netns-fixed:v1"}}
        }
      },
      "edges": {
        "cross-boundary": {
          "producer": {"child": "edgehost", "path": "/run/netns-edge"},
          "consumers": {"a/edgeclient": {}}
        }
      }
    }
  '';
in
scenario.nodeWith {
  networking.nameservers = [ "10.231.0.1" ];
  networking.useNetworkd = true;
} ''
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${fixedServer}) scenario-netns-fixed:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${fdServer}) scenario-netns-fd:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${egressProbe}) scenario-netns-egress:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${edgeProducer}) scenario-netns-edge-producer:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${edgeConsumer}) scenario-netns-edge-consumer:v1")
  machine.succeed("cp ${root} /tmp/scenario/cix.json")
  machine.succeed("systemd-run --unit=scenario-dns ${python}/bin/python3 ${dnsServer}")
  machine.succeed("systemd-run --unit=scenario-egress-target ${python}/bin/python3 -m http.server 19090 --bind 0.0.0.0")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json")

  machine.wait_until_succeeds("ip netns exec cix-netns-a-netns curl -fsS http://127.0.0.1:8080 | grep -Fx fixed-pod")
  machine.wait_until_succeeds("ip netns exec cix-netns-b-netns curl -fsS http://127.0.0.1:8080 | grep -Fx fixed-pod")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080 | grep -Fx v1")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18081 | grep -Fx fixed-pod")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18082 | grep -Fx edge-ok")
  machine.wait_until_succeeds("test $(cat /run/cix-netns-a-allowed/run/netns-probe/result) = allowed")
  machine.wait_until_succeeds("test $(cat /run/cix-netns-a-denied/run/netns-probe/result) = denied")
  machine.succeed("systemctl cat cix-netns-publish-a-fd.socket | grep -F 'ListenStream=127.0.0.1:18080'")
  machine.succeed("systemctl cat cix-netns-a-fd.service | grep -F 'NetworkNamespacePath=/run/netns/cix-netns-a-netns'")
  machine.succeed("test -z \"$(ip -o -6 address show dev cix0 scope link)\"")
  machine.succeed("test -z \"$(ip -n cix-netns-a-netns -o -6 address show scope link)\"")
  machine.succeed("test -z \"$(ip -n cix-netns-b-netns -o -6 address show scope link)\"")

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down netns")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json --closed-root")
  machine.wait_until_succeeds("test $(cat /run/cix-netns-a-allowed/run/netns-probe/result) = allowed")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18082 | grep -Fx edge-ok")
  machine.succeed("systemctl cat cix-netns-a-allowed.service | grep -F 'BindReadOnlyPaths=/run/systemd/resolve/resolv.conf:/etc/resolv.conf'")
  machine.succeed("systemctl cat cix-netns-a-denied.service | grep -F 'IPAddressDeny=any'")
  machine.succeed("systemctl cat cix-netns-a-allowed.service | grep -F 'NetworkNamespacePath=/run/netns/cix-netns-a-netns'")
  machine.succeed("test \"$(systemctl show cix-netns-b-netns.service --property=TimeoutStopUSec --value)\" = 10s")

  lease = machine.succeed("jq -r '.leases[\"netns/a\"].host' /var/lib/cix-compose/ipam.json").strip()
  machine.succeed("sed -i 's/\\\"MESSAGE\\\": \\\"v1\\\"/\\\"MESSAGE\\\": \\\"v2\\\"/' /tmp/scenario/cix.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json --closed-root")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080 | grep -Fx v2")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix rollback netns")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080 | grep -Fx v1")
  machine.succeed("test $(jq -r '.leases[\"netns/a\"].host' /var/lib/cix-compose/ipam.json) = " + lease)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json --closed-root")
  machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080 | grep -Fx v2")
  machine.succeed("test $(jq -r '.leases[\"netns/a\"].host' /var/lib/cix-compose/ipam.json) = " + lease)

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down netns")
  machine.succeed("systemctl stop scenario-dns.service")
  machine.succeed("systemctl stop scenario-egress-target.service")
  machine.succeed("test ! -e /run/netns/cix-netns-a-netns")
  machine.succeed("test ! -e /run/netns/cix-netns-b-netns")
  machine.succeed("test -n $(jq -r '.leases[\"netns/a\"].host' /var/lib/cix-compose/ipam.json)")
  machine.succeed("systemctl reset-failed 'cix-netns*' || true")
  machine.succeed("test -z \"$(systemctl list-units --all --no-legend 'cix-netns*' | awk 'NF { print $1 }')\"")
''
