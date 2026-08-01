{
  pkgs ? import <nixpkgs> { },
  composix ? import ../../../nix/lib.nix { inherit pkgs; },
}:

let
  listenerProgram = pkgs.runCommand "listenfds-program" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/listenfds" <<'EOF'
    #!${pkgs.python3}/bin/python3
    import os
    import socket

    if os.environ.get("LISTEN_FDS") != "1":
        raise SystemExit("expected exactly one inherited listener")
    if os.environ.get("LISTEN_FDNAMES") != "http":
        raise SystemExit("expected inherited listener named http")

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
            body = b"LISTEN_FDS=1; no socket() authority\n"
            connection.sendall(
                b"HTTP/1.1 200 OK\r\n"
                + b"Content-Type: text/plain\r\n"
                + f"Content-Length: {len(body)}\r\n".encode()
                + b"Connection: close\r\n\r\n"
                + body
            )
    EOF
    chmod +x "$out/bin/listenfds"
  '';
in
composix.withSpec {
  name = "listenfds-cix";
  manifest = {
    cixManifest = 0;
    start = [ "opt/listenfds/bin/listenfds" ];
    mounts = [ "/opt/listenfds" ];
    listeners.http.type = "stream";
  };
  mounts."/opt/listenfds" = listenerProgram;
}
