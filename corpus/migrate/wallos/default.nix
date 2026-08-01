{
  system ? builtins.currentSystem,
  pkgs ? import (builtins.fetchTree {
    type = "github";
    owner = "NixOS";
    repo = "nixpkgs";
    rev = "1559d3daa3ecc813a650b79375ea61b6741b8746";
    narHash = "sha256-LQy14TZp77TwbQf40gg1V3jo8FwJG0jGDkAH+zRHqg8=";
  }) { inherit system; },
  composix ? import ../../../nix/lib.nix { inherit pkgs; },
}:

let
  php = pkgs.php83.withExtensions (
    { enabled, all }:
    enabled
    ++ (with all; [
      calendar
      gd
      intl
      pdo_sqlite
      zip
    ])
  );

  app = pkgs.runCommand "wallos-app" { } ''
    cp -R ${./context}/. "$out"
    chmod -R u+w "$out"
    rm -rf "$out/db" "$out/images/uploads/logos"
    ln -s /var/lib/wallos/db "$out/db"
    mkdir -p "$out/images/uploads"
    ln -s /var/lib/wallos/logos "$out/images/uploads/logos"
    rm -f "$out/Dockerfile" "$out/nginx.conf" "$out/nginx.default.conf" "$out/startup.sh"
  '';

  cronjobs = pkgs.runCommand "wallos-cronjobs" { } ''
    tr -d '\r' < ${./context/cronjobs} \
      | sed 's#/usr/local/bin/php#${php}/bin/php#g; s#/var/log/cron#/var/log/wallos#g' \
      > "$out"
  '';

  phpFpmConfig = pkgs.writeText "wallos-php-fpm.conf" ''
    [global]
    error_log = /var/log/wallos/php-fpm.log
    daemonize = no

    [www]
    listen = /run/wallos/php-fpm.sock
    listen.mode = 0660
    pm = dynamic
    pm.max_children = 15
    pm.start_servers = 2
    pm.min_spare_servers = 1
    pm.max_spare_servers = 3
    pm.max_requests = 500
    catch_workers_output = yes
    clear_env = no
  '';

  nginxConfig = pkgs.writeText "wallos-nginx.conf" ''
    daemon off;
    pid /run/wallos/nginx.pid;
    error_log stderr info;
    events { worker_connections 1024; }
    http {
      include ${pkgs.nginx}/conf/mime.types;
      default_type application/octet-stream;
      access_log /var/log/wallos/nginx-access.log;
      client_body_temp_path /var/cache/wallos/body;
      proxy_temp_path /var/cache/wallos/proxy;
      fastcgi_temp_path /var/cache/wallos/fastcgi;
      uwsgi_temp_path /var/cache/wallos/uwsgi;
      scgi_temp_path /var/cache/wallos/scgi;
      server {
        listen 18092;
        root /var/www/html;
        index index.php;
        location / { try_files $uri $uri/ /index.php?$args; }
        location ~ \.php$ {
          include ${pkgs.nginx}/conf/fastcgi_params;
          fastcgi_pass unix:/run/wallos/php-fpm.sock;
          fastcgi_index index.php;
          fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        }
        location ~ \.db$ { deny all; return 403; }
        location ~* images/uploads/logos/.*\.php$ { deny all; return 403; }
        location ~* \.tmp/.*\.php$ { deny all; return 403; }
        location ~* ^/includes/.*\.php$ { deny all; return 403; }
      }
    }
  '';

  setup = pkgs.writeShellScript "wallos-setup" ''
    set -eu
    ${pkgs.coreutils}/bin/mkdir -p /var/lib/wallos/db /var/lib/wallos/logos
    cd /var/www/html
    ${php}/bin/php endpoints/cronjobs/createdatabase.php
    ${php}/bin/php endpoints/db/migrate.php
    ${php}/bin/php endpoints/cronjobs/updatenextpayment.php
    ${php}/bin/php endpoints/cronjobs/updateexchange.php
    ${php}/bin/php endpoints/cronjobs/checkforupdates.php
  '';

  start = pkgs.writeShellScript "wallos-start" ''
    set -eu
    php_pid=
    cron_pid=
    nginx_pid=
    cleanup() {
      test -z "$nginx_pid" || ${pkgs.coreutils}/bin/kill -QUIT "$nginx_pid" 2>/dev/null || true
      test -z "$php_pid" || ${pkgs.coreutils}/bin/kill -QUIT "$php_pid" 2>/dev/null || true
      test -z "$cron_pid" || ${pkgs.coreutils}/bin/kill -TERM "$cron_pid" 2>/dev/null || true
    }
    trap cleanup EXIT INT TERM QUIT
    ${php}/bin/php-fpm -y ${phpFpmConfig} -F &
    php_pid=$!
    ${pkgs.supercronic}/bin/supercronic ${cronjobs} &
    cron_pid=$!
    ${pkgs.nginx}/bin/nginx -c ${nginxConfig} -e stderr &
    nginx_pid=$!
    wait -n "$php_pid" "$cron_pid" "$nginx_pid"
  '';

  mounts = {
    "/var/www/html" = app;
  };
in
composix.withSpec {
  name = "cix-item-wallos";
  inherit mounts;
  manifest = {
    cixManifest = 0;
    exec = [ start ];
    setup = [ setup ];
    mounts = builtins.attrNames mounts;
    ports.http = {
      protocol = "tcp";
      value = 18092;
    };
    dirs = {
      state = [ "/var/lib/wallos" ];
      cache = [ "/var/cache/wallos" ];
      logs = [ "/var/log/wallos" ];
      run = [ "/run/wallos" ];
    };
    claims = [
      "egress"
      "jit"
    ];
  };
}
