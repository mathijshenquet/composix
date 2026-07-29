# First-contact dogfood: plain nginx serving a static page, spec'd for cix run.
# Build: nix-build examples/nginx -o result-nginx
{ pkgs ? import <nixpkgs> { } }:

let
  html = pkgs.writeText "index.html" ''
    <h1>hello from composix</h1>
  '';

  conf = pkgs.writeText "nginx.conf" ''
    daemon off;
    pid /run/nginx/nginx.pid;
    error_log stderr info;
    events { }
    http {
      include /etc/nginx/mime.types;
      access_log off;
      client_body_temp_path /var/cache/nginx/body;
      proxy_temp_path /var/cache/nginx/proxy;
      fastcgi_temp_path /var/cache/nginx/fastcgi;
      uwsgi_temp_path /var/cache/nginx/uwsgi;
      scgi_temp_path /var/cache/nginx/scgi;
      server {
        listen 8080;
        root /srv/www;
      }
    }
  '';
in
pkgs.runCommand "nginx-cix" { } ''
  mkdir -p $out/etc/nginx $out/srv/www
  ln -s ${pkgs.nginx}/conf/mime.types $out/etc/nginx/mime.types
  install -m 0644 ${html} $out/srv/www/index.html
  install -m 0644 ${conf} $out/etc/nginx/nginx.conf
  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 2,
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
''
