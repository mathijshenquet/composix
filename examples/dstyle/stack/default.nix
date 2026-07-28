{ pkgs ? import <nixpkgs> { } }:

let
  nginxConf = pkgs.writeText "dstyle-stack-nginx.conf" ''
    daemon off;
    pid /run/nginx/nginx.pid;
    error_log stderr info;
    events { }
    http {
      access_log off;
      client_body_temp_path /var/cache/nginx/body;
      proxy_temp_path /var/cache/nginx/proxy;
      fastcgi_temp_path /var/cache/nginx/fastcgi;
      uwsgi_temp_path /var/cache/nginx/uwsgi;
      scgi_temp_path /var/cache/nginx/scgi;
      server {
        listen unix:/run/nginx/http.sock;
        location / {
          proxy_pass http://unix:/run/stack-shared/backend.sock:;
        }
      }
    }
  '';
in
pkgs.runCommand "dstyle-unix-stack" { } ''
  mkdir -p $out/bin
  ln -s ${pkgs.nginx}/bin/nginx $out/bin/nginx
  ln -s ${nginxConf} $out/nginx.conf

  cat > $out/bin/backend <<'EOF'
  #!${pkgs.python3}/bin/python3
  import os
  import socket

  path = "/run/backend/backend.sock"
  try:
      os.unlink(path)
  except FileNotFoundError:
      pass

  server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
  server.bind(path)
  os.chmod(path, 0o660)
  server.listen()

  while True:
      connection, _ = server.accept()
      with connection:
          request = b""
          while b"\r\n\r\n" not in request:
              chunk = connection.recv(4096)
              if not chunk:
                  break
              request += chunk
          body = b"hello from the dstyle backend\n"
          connection.sendall(
              b"HTTP/1.1 200 OK\r\n"
              + b"Content-Type: text/plain\r\n"
              + f"Content-Length: {len(body)}\r\n".encode()
              + b"Connection: close\r\n\r\n"
              + body
          )
  EOF
  chmod +x $out/bin/backend

  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 2,
    "services": {
      "backend": {
        "exec": ["bin/backend"],
        "dirs": {
          "run": ["/run/backend"]
        }
      },
      "nginx": {
        "exec": ["bin/nginx", "-c", "nginx.conf", "-e", "stderr"],
        "dirs": {
          "cache": ["/var/cache/nginx"],
          "run": ["/run/nginx"]
        }
      }
    }
  }
  EOF
''

