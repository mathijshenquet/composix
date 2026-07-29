{ pkgs ? import <nixpkgs> { } }:

pkgs.runCommand "dstyle-listenfds" { } ''
  mkdir -p $out/bin
  cat > $out/bin/listenfds <<'EOF'
  #!${pkgs.python3}/bin/python3
  import os
  import socket

  listen_fds = int(os.environ.get("LISTEN_FDS", "0"))
  listen_pid = int(os.environ.get("LISTEN_PID", "0"))
  if listen_fds != 1 or listen_pid != os.getpid():
      raise SystemExit(
          f"expected one systemd listener for pid {os.getpid()}, "
          f"got LISTEN_PID={listen_pid} LISTEN_FDS={listen_fds}"
      )

  listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
  while True:
      connection, _ = listener.accept()
      with connection:
          request = b""
          while b"\r\n\r\n" not in request:
              chunk = connection.recv(4096)
              if not chunk:
                  break
              request += chunk
          body = f"LISTEN_FDS={listen_fds}; no socket() authority\n".encode()
          connection.sendall(
              b"HTTP/1.1 200 OK\r\n"
              + b"Content-Type: text/plain\r\n"
              + f"Content-Length: {len(body)}\r\n".encode()
              + b"Connection: close\r\n\r\n"
              + body
          )
  EOF
  chmod +x $out/bin/listenfds

  cat > $out/cix-manifest.json <<'EOF'
  {
    "cixManifest": 2,
    "services": {
      "listenfds": {
        "exec": ["bin/listenfds"]
      }
    }
  }
  EOF
''

