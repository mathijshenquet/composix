{ pkgs ? import <nixpkgs> { } }:

let
  html = pkgs.writeTextDir "index.html" ''
    <h1>hello from dstyle nginx</h1>
  '';

  conf = pkgs.writeText "nginx-dstyle.conf" ''
    daemon off;
    pid /run/nginx/nginx.pid;
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
        listen unix:/run/nginx/http.sock;
        root ${html};
      }
    }
  '';
in
pkgs.runCommand "dstyle-nginx-unix" { } ''
  mkdir -p $out/bin
  ln -s ${pkgs.nginx}/bin/nginx $out/bin/nginx
  cat > $out/cix-manifest.json <<'EOF'
  {
    "cixManifest": 0,
    "start": ["bin/nginx", "-c", "${conf}", "-e", "stderr"],
    "dirs": {
      "cache": ["/var/cache/nginx"],
      "run": ["/run/nginx"]
    }
  }
  EOF
''
