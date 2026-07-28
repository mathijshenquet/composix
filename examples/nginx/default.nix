# First-contact dogfood: plain nginx serving a static page, spec'd for cix run.
# Build: nix-build examples/nginx -o result-nginx
{ pkgs ? import <nixpkgs> { } }:

let
  html = pkgs.writeTextDir "index.html" ''
    <h1>hello from composix</h1>
  '';

  conf = pkgs.writeText "nginx.conf" ''
    daemon off;
    pid /var/cache/nginx/nginx.pid;
    error_log stderr info;
    events { }
    http {
      include ${pkgs.nginx}/conf/mime.types;
      access_log off;
      client_body_temp_path /var/cache/nginx/body;
      proxy_temp_path /var/cache/nginx/proxy;
      fastcgi_temp_path /var/cache/nginx/fastcgi;
      uwsgi_temp_path /var/cache/nginx/uwsgi;
      scgi_temp_path /var/cache/nginx/scgi;
      server {
        listen 8080;
        root ${html};
      }
    }
  '';
in
pkgs.runCommand "nginx-cix" { } ''
  mkdir -p $out/bin
  ln -s ${pkgs.nginx}/bin/nginx $out/bin/nginx
  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 1,
    "services": {
      "nginx": {
        "exec": ["bin/nginx", "-c", "@conf@", "-e", "stderr"],
        "env": { "PORT": { "type": "port", "default": 8080 } },
        "ports": { "http": { "env": "PORT", "protocol": "tcp" } },
        "dirs": { "cache": ["/var/cache/nginx"] }
      }
    }
  }
  EOF
  substituteInPlace $out/cix-spec.json --replace-fail '@conf@' '${conf}'
''
