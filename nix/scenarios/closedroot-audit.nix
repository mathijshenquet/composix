{ pkgs, cix }:

let
  composix = import ../lib.nix { inherit pkgs; };
  scenario = import ./lib.nix { inherit pkgs cix; };

  port = value: {
    inherit value;
    protocol = "tcp";
  };

  item = name: manifest: mounts: composix.withSpec {
    name = "closedroot-audit-${name}";
    inherit manifest mounts;
  };

  webRoot = name: source: pkgs.runCommand "closedroot-audit-${name}-www" { } ''
    install -Dm0644 ${source} "$out/index.html"
  '';

  caddyPack = item "pack-caddy" {
    cixManifest = 0;
    start = [ "${pkgs.caddy}/bin/caddy" "file-server" "--root" "/srv/www" "--listen" ":8080" ];
    mounts = [ "/srv/www" ];
    ports.http = port 8080;
  } {
    "/srv/www" = webRoot "pack-caddy" ../../examples/pack/caddy/index.html;
  };

  deviceProgram = pkgs.runCommand "closedroot-audit-device-program" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/start" <<'SH'
    #!${pkgs.runtimeShell}
    set -eu
    ${pkgs.coreutils}/bin/head -c 1 /dev/cix-device > /dev/null
    ${pkgs.util-linux}/bin/findmnt -n -o OPTIONS --target /dev/shm | ${pkgs.gnugrep}/bin/grep -E '(^|,)size=(64M|65536k)(,|$)'
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/start"
  '';
  devicesPack = item "pack-devices" {
    cixManifest = 0;
    start = [ "bin/start" ];
    claims = [ { device = "/dev/cix-device"; } ];
    shm = "64M";
  } {
    "/bin/start" = "${deviceProgram}/bin/start";
  };

  listenfdsPack = import ../../examples/pack/listenfds {
    inherit pkgs composix;
  };

  nginxConfig = pkgs.runCommand "closedroot-audit-nginx-config" { } ''
    install -Dm0644 ${../../examples/pack/nginx/nginx.conf} "$out/nginx.conf"
    ln -s ${pkgs.nginx}/conf/mime.types "$out/mime.types"
  '';
  nginxPack = item "pack-nginx" {
    cixManifest = 0;
    start = [ "${pkgs.nginx}/bin/nginx" "-c" "/etc/nginx/nginx.conf" "-e" "stderr" ];
    mounts = [ "/etc/nginx" "/srv/www" ];
    ports.http = port 8080;
    dirs = {
      cache = [ "/var/cache/nginx" ];
      run = [ "/run/nginx" ];
    };
  } {
    "/etc/nginx" = nginxConfig;
    "/srv/www" = webRoot "pack-nginx" ../../examples/pack/nginx/index.html;
  };

  nodePack = item "pack-node-app" {
    cixManifest = 0;
    start = [ "${pkgs.nodejs}/bin/node" "/app/server.js" ];
    mounts = [ "/app" ];
    ports.http = port 8081;
    claims = [ "jit" ];
  } {
    "/app" = pkgs.runCommand "closedroot-audit-node-app" { } ''
      install -Dm0644 ${../../examples/pack/node-app/server.js} "$out/server.js"
    '';
  };

  postgresTemplate =
    (pkgs.extend (import ../../examples/pack/postgres/postgres.nix)).composixPostgresTemplate;
  postgresPayload = pkgs.runCommand "closedroot-audit-postgres-payload" { } ''
    mkdir -p "$out"
    install -m0644 ${../../examples/pack/postgres/runtime-env.sh} "$out/runtime-env.sh"
    install -m0755 ${../../examples/pack/postgres/setup} "$out/setup"
    install -m0755 ${../../examples/pack/postgres/start} "$out/start"
    ln -s ${pkgs.nss_wrapper}/lib/libnss_wrapper.so "$out/libnss_wrapper.so"
    cp -R ${postgresTemplate} "$out/template"
  '';
  postgresPack = item "pack-postgres" {
    cixManifest = 0;
    start = [ "${pkgs.bash}/bin/sh" "/opt/postgres/start" "$PORT" ];
    start_pre = [ "${pkgs.bash}/bin/sh" "/opt/postgres/setup" ];
    mounts = [ "/opt/postgres" ];
    env = {
      PATH.default = "${pkgs.postgresql}/bin:${pkgs.coreutils}/bin";
      PORT.default = "5432";
    };
    ports.postgres = {
      env = "PORT";
      protocol = "tcp";
    };
    dirs = {
      state = [ "/var/lib/postgresql" ];
      run = [ "/run/postgresql" ];
    };
  } {
    "/opt/postgres" = postgresPayload;
  };

  redisPack = import ../../examples/pack/redis {
    inherit pkgs composix;
  };

  adminerCorpus = item "corpus-adminer" {
    cixManifest = 0;
    start = [
      "${pkgs.php84}/bin/php"
      "-S"
      "0.0.0.0:8080"
      "-t"
      "${pkgs.adminer}"
      "${pkgs.adminer}/adminer.php"
    ];
    ports.http = port 8080;
  } { };

  caddyCorpus = item "corpus-caddy" {
    cixManifest = 0;
    start = [ "${pkgs.caddy}/bin/caddy" "respond" "--listen" ":8080" "Caddy" ];
    env = {
      XDG_CONFIG_HOME.default = "/var/lib/caddy/config";
      XDG_DATA_HOME.default = "/var/lib/caddy/data";
    };
    ports.http = port 8080;
    dirs.state = [ "/var/lib/caddy" ];
  } { };

  memcachedCorpus = item "corpus-memcached" {
    cixManifest = 0;
    start = [ "${pkgs.memcached}/bin/memcached" "-l" "0.0.0.0" ];
    ports.memcached = port 11211;
  } { };

  natsCorpus = item "corpus-nats" {
    cixManifest = 0;
    start = [ "${pkgs.nats-server}/bin/nats-server" "-m" "8222" ];
    ports = {
      client = port 4222;
      monitor = port 8222;
    };
  } { };

  nginxCorpusConfig = pkgs.writeText "closedroot-audit-corpus-nginx.conf" ''
    daemon off;
    events { }
    http {
      access_log off;
      server {
        listen 8080;
        location / { return 200 "nginx\n"; }
      }
    }
  '';
  nginxCorpus = item "corpus-nginx" {
    cixManifest = 0;
    start = [ "${pkgs.nginx}/bin/nginx" "-c" "/etc/nginx/nginx.conf" "-e" "stderr" ];
    mounts = [ "/etc/nginx/nginx.conf" ];
    ports.http = port 8080;
    dirs = {
      logs = [ "/var/log/nginx" ];
      run = [ "/run/nginx" ];
    };
  } {
    "/etc/nginx/nginx.conf" = nginxCorpusConfig;
  };

  phpmyadminSource = pkgs.fetchurl {
    url = "https://files.phpmyadmin.net/phpMyAdmin/5.2.3/phpMyAdmin-5.2.3-all-languages.tar.xz";
    hash = "sha256-V4gTSCl8RBL4bEEFR892tNiiNldN0sa31qK+6+f8ROM=";
  };
  phpmyadminTree = pkgs.runCommand "closedroot-audit-phpmyadmin-tree" {
    nativeBuildInputs = [ pkgs.xz ];
  } ''
    mkdir "$out"
    tar -xJf ${phpmyadminSource} --strip-components=1 -C "$out"
  '';
  phpmyadminCorpus = item "corpus-phpmyadmin" {
    cixManifest = 0;
    start = [ "${pkgs.php83}/bin/php" "-S" "0.0.0.0:8080" "-t" "${phpmyadminTree}" ];
    ports.http = port 8080;
    dirs.state = [ "/var/lib/phpmyadmin" ];
  } { };

  redisCorpus = item "corpus-redis" {
    cixManifest = 0;
    start = [ "${pkgs.redis}/bin/redis-server" "--dir" "/data" "--port" "6379" ];
    env = {
      LANG.default = "C";
      LC_ALL.default = "C";
    };
    ports.redis = port 6379;
    dirs.state = [ "/data" ];
  } { };

  renovateCorpus = item "corpus-renovate" {
    cixManifest = 0;
    kind = "app";
    start = [ "${pkgs.renovate}/bin/renovate" "--version" ];
    claims = [ "jit" ];
  } { };

  tomcatCorpus = item "corpus-tomcat" {
    cixManifest = 0;
    start = [ "${pkgs.bash}/bin/sh" "/tomcat/bin/catalina.sh" "run" ];
    start_pre = [ "${pkgs.bash}/bin/sh" "/bin/tomcat-setup" ];
    mounts = [ "/bin/tomcat-setup" "/coreutils" "/gnused" "/tomcat" "/jre" ];
    env = {
      CATALINA_HOME.default = "/tomcat";
      CATALINA_BASE.default = "/var/lib/tomcat";
      CATALINA_TMPDIR.default = "/run/tomcat";
      JRE_HOME.default = "/jre";
      PATH.default = "/coreutils/bin:/gnused/bin:/bin";
    };
    ports.http = port 8080;
    claims = [ "jit" ];
    dirs = {
      state = [ "/var/lib/tomcat" ];
      logs = [ "/var/log/tomcat" ];
      run = [ "/run/tomcat" ];
    };
  } {
    "/bin/tomcat-setup" = ../../corpus/migrate/docker/tomcat/setup.sh;
    "/coreutils" = pkgs.coreutils;
    "/gnused" = pkgs.gnused;
    "/tomcat" = pkgs.tomcat10;
    "/jre" = pkgs.jdk21_headless;
  };

  traefikCorpus = item "corpus-traefik" {
    cixManifest = 0;
    start = [ "${pkgs.traefik}/bin/traefik" "--ping=true" "--entryPoints.web.address=:8081" ];
    ports = {
      http = port 8081;
      monitor = port 8080;
    };
  } { };

  mastodonPostgresPayload = pkgs.runCommand "closedroot-audit-mastodon-postgres" { } ''
    mkdir -p "$out"
    install -m0644 ${../../corpus/migrate/docker/mastodon/postgres/runtime-env.sh} "$out/runtime-env.sh"
    install -m0644 ${../../corpus/migrate/docker/mastodon/postgres/setup.sh} "$out/setup.sh"
    install -m0644 ${../../corpus/migrate/docker/mastodon/postgres/start.sh} "$out/start.sh"
    ln -s ${pkgs.nss_wrapper}/lib/libnss_wrapper.so "$out/libnss_wrapper.so"
  '';
  mastodonPostgres = item "corpus-mastodon-postgres" {
    cixManifest = 0;
    start = [
      "${pkgs.bash}/bin/sh"
      "/opt/postgres/start.sh"
      "${pkgs.postgresql}/bin/postgres"
    ];
    start_pre = [ "${pkgs.bash}/bin/sh" "/opt/postgres/setup.sh" ];
    mounts = [ "/bin/sh" "/opt/postgres" ];
    env = {
      LANG.default = "C";
      LC_ALL.default = "C";
      PATH.default = "${pkgs.postgresql}/bin:${pkgs.coreutils}/bin";
    };
    secrets.db-password = { };
    ports.postgres = port 35432;
    dirs = {
      state = [ "/var/lib/postgresql" ];
      run = [ "/run/postgresql" ];
    };
    readiness = {
      type = "tcp";
      target = ":35432";
      timeout = "30s";
    };
  } {
    "/bin/sh" = "${pkgs.bash}/bin/sh";
    "/opt/postgres" = mastodonPostgresPayload;
  };

  mastodonRedisConfig = pkgs.runCommand "closedroot-audit-mastodon-redis" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/redis/redis.conf} "$out/redis.conf"
  '';
  mastodonRedis = item "corpus-mastodon-redis" {
    cixManifest = 0;
    start = [ "${pkgs.redis}/bin/redis-server" "/etc/redis/redis.conf" ];
    mounts = [ "/etc/redis" ];
    env = {
      LANG.default = "C";
      LC_ALL.default = "C";
      PATH.default = "bin";
    };
    ports.redis = port 36379;
    dirs.run = [ "/run/redis" ];
    readiness = {
      type = "tcp";
      target = ":36379";
      timeout = "20s";
    };
  } {
    "/etc/redis" = mastodonRedisConfig;
  };

  mastodonWebPayload = pkgs.runCommand "closedroot-audit-mastodon-web" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/web/web.py} "$out/web.py"
  '';
  mastodonWeb = item "corpus-mastodon-web" {
    cixManifest = 0;
    start = [
      "${pkgs.python3}/bin/python3"
      "/opt/mastodon/web.py"
      "${pkgs.postgresql}/bin/psql"
      "${pkgs.redis}/bin/redis-cli"
    ];
    mounts = [ "/opt/mastodon" ];
    env.PATH.default = "bin";
    secrets.db-password = { };
    ports.http = port 33000;
    dirs.state = [ "/mastodon/public/system" ];
    readiness = {
      type = "http";
      target = ":33000/health";
      timeout = "30s";
    };
    liveness = {
      type = "notify";
      interval = "2s";
    };
    claims = [ "egress" ];
  } {
    "/opt/mastodon" = mastodonWebPayload;
  };

  mastodonSidekiqPayload = pkgs.runCommand "closedroot-audit-mastodon-sidekiq" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/sidekiq/worker.py} "$out/worker.py"
  '';
  mastodonSidekiq = item "corpus-mastodon-sidekiq" {
    cixManifest = 0;
    start = [
      "${pkgs.python3}/bin/python3"
      "/opt/mastodon/worker.py"
      "${pkgs.postgresql}/bin/psql"
      "${pkgs.redis}/bin/redis-cli"
    ];
    mounts = [ "/opt/mastodon" ];
    env.PATH.default = "bin";
    secrets.db-password = { };
    ports.control = port 33001;
    dirs.state = [ "/mastodon/public/system" ];
    readiness = {
      type = "tcp";
      target = ":33001";
      timeout = "30s";
    };
    liveness = {
      type = "notify";
      interval = "2s";
    };
    claims = [ "egress" ];
  } {
    "/opt/mastodon" = mastodonSidekiqPayload;
  };

  mastodonStreamingConfig = pkgs.runCommand "closedroot-audit-mastodon-streaming-nginx" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/streaming/nginx.conf} "$out/nginx.conf"
  '';
  mastodonStreamingPayload = pkgs.runCommand "closedroot-audit-mastodon-streaming" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/streaming/verify-dependencies.sh} "$out/verify-dependencies.sh"
  '';
  mastodonStreaming = item "corpus-mastodon-streaming" {
    cixManifest = 0;
    start = [
      "${pkgs.nginx}/bin/nginx"
      "-c"
      "/etc/nginx/nginx.conf"
      "-e"
      "stderr"
    ];
    start_pre = [
      "${pkgs.bash}/bin/sh"
      "/opt/mastodon/verify-dependencies.sh"
      "${pkgs.postgresql}/bin/psql"
      "${pkgs.redis}/bin/redis-cli"
    ];
    mounts = [ "/etc/nginx" "/opt/mastodon" ];
    env.PATH.default = "bin";
    secrets.db-password = { };
    ports.streaming = port 34000;
    dirs = {
      cache = [ "/var/cache/nginx" ];
      run = [ "/run/nginx" ];
    };
    readiness = {
      type = "http";
      target = ":34000/api/v1/streaming/health";
      timeout = "30s";
    };
  } {
    "/etc/nginx" = mastodonStreamingConfig;
    "/opt/mastodon" = mastodonStreamingPayload;
  };

  mastodonCleanupPayload = pkgs.runCommand "closedroot-audit-mastodon-cleanup" { } ''
    install -Dm0644 ${../../corpus/migrate/docker/mastodon/cleanup/cleanup.sh} "$out/cleanup.sh"
  '';
  mastodonCleanup = item "corpus-mastodon-cleanup" {
    cixManifest = 0;
    kind = "app";
    start = [
      "${pkgs.bash}/bin/sh"
      "/opt/mastodon/cleanup.sh"
      "${pkgs.postgresql}/bin/psql"
    ];
    mounts = [ "/opt/mastodon" ];
    env.PATH.default = "bin";
    secrets.db-password = { };
  } {
    "/opt/mastodon" = mastodonCleanupPayload;
  };

  mastodonCompose = pkgs.writeText "closedroot-audit-mastodon.json" (builtins.toJSON {
    cixCompose = 1;
    name = "closedroot-mastodon";
    children = {
      postgres.item = "closedroot-mastodon-postgres:v1";
      redis.item = "closedroot-mastodon-redis:v1";
      web = {
        item = "closedroot-mastodon-web:v1";
        dirs."/mastodon/public/system".shared = "public-system";
      };
      sidekiq = {
        item = "closedroot-mastodon-sidekiq:v1";
        dirs."/mastodon/public/system".shared = "public-system";
      };
      streaming.item = "closedroot-mastodon-streaming:v1";
      cleanup = {
        item = "closedroot-mastodon-cleanup:v1";
        schedule = "*-*-* *:*:00/5";
        persistent = true;
      };
    };
    edges = {
      database = {
        producer = {
          child = "postgres";
          path = "/run/postgresql";
        };
        consumers = {
          web = { };
          sidekiq = { };
          streaming = { };
          cleanup = { };
        };
      };
      redis = {
        producer = {
          child = "redis";
          path = "/run/redis";
        };
        consumers = {
          web = { };
          sidekiq = { };
          streaming = { };
        };
      };
    };
    secrets.db-password.file = "/run/closedroot-mastodon-secret";
  });

  auditedPacks = [ "caddy" "devices" "listenfds" "nginx" "node-app" "postgres" "redis" ];
  auditedCorpus = [ "docker/adminer" "docker/caddy" "docker/mastodon" "docker/memcached" "docker/nats" "docker/nginx" "docker/phpmyadmin" "docker/redis" "docker/renovate" "docker/tomcat" "docker/traefik" ];
  downgradedCorpus = [ "docker/directus" "docker/dozzle" "docker/echo-server" "docker/excalidraw" "docker/filestash" "docker/parse-server" "docker/verdaccio" "docker/wallos" "docker/watchtower" "docker/whoami" ];
  packNames = builtins.attrNames (pkgs.lib.filterAttrs (_: type: type == "directory") (builtins.readDir ../../examples/pack));
  corpusAxes = builtins.attrNames (pkgs.lib.filterAttrs (_: type: type == "directory") (builtins.readDir ../../corpus/migrate));
  corpusNames = builtins.concatMap (axis:
    map (name: "${axis}/${name}")
      (builtins.attrNames (pkgs.lib.filterAttrs (_: type: type == "directory") (builtins.readDir ../../corpus/migrate/${axis})))) corpusAxes;
  inventoryComplete =
    assert pkgs.lib.sort builtins.lessThan auditedPacks == packNames;
    assert pkgs.lib.sort builtins.lessThan corpusAxes == [ "docker" "k8s" ];
    assert pkgs.lib.sort builtins.lessThan (auditedCorpus ++ downgradedCorpus) == corpusNames;
    true;
in
assert inventoryComplete;
scenario.node ''
  import time

  machine.succeed("mknod -m 666 /dev/cix-device c 1 3")

  def assert_closed_unit(unit, declared_shell=False):
      root = machine.succeed("systemctl show " + unit + " --property=RootDirectory --value").strip()
      assert root.startswith("/run/cix/closed-roots/")
      machine.succeed("systemctl show " + unit + " --property=MountAPIVFS --value | grep -Fx yes")
      machine.succeed("systemctl show " + unit + " --property=BindReadOnlyPaths --value | grep -F /nix/store")
      machine.succeed("test -L " + root + "/usr/bin/env")
      if declared_shell:
          machine.succeed("systemctl show " + unit + " --property=BindReadOnlyPaths --value | grep -F ':/bin/sh'")
      else:
          machine.succeed("test ! -e " + root + "/bin/sh")
      return root

  def audit(item, probe, port_override=None):
      added_item = machine.succeed("nix-store --add " + item).strip()
      command = "cix run --closed-root --detach " + added_item
      if port_override is not None:
          command += " -p " + port_override
      unit = machine.succeed(command).strip().splitlines()[-1]
      machine.succeed("systemctl is-active " + unit)
      assert_closed_unit(unit)
      machine.wait_until_succeeds(probe, timeout=90)
      machine.succeed("systemctl kill --kill-whom=main --signal=KILL " + unit)
      machine.wait_until_succeeds("test ! -e /run/cix/gcroots/" + unit + ".root")
      machine.succeed("systemctl reset-failed " + unit + " || true")

  renovate_item = machine.succeed("nix-store --add ${renovateCorpus}").strip()
  renovate = machine.succeed("cix run --closed-root " + renovate_item)
  assert renovate.strip()
  audit("${nginxCorpus}", "curl --fail --silent http://127.0.0.1:8080/ | grep -Fx nginx")

  audit("${postgresPack}", "${pkgs.postgresql}/bin/pg_isready --host 127.0.0.1 --port 5432")
  audit("${redisPack}", "${pkgs.redis}/bin/redis-cli -h 127.0.0.1 -p 6379 ping | grep -Fx PONG")
  audit("${redisCorpus}", "${pkgs.redis}/bin/redis-cli -h 127.0.0.1 -p 6379 ping | grep -Fx PONG")
  audit("${memcachedCorpus}", "printf 'version\\r\\n' | ${pkgs.netcat-openbsd}/bin/nc -w 2 127.0.0.1 11211 | grep -E '^VERSION '")
  audit("${natsCorpus}", "curl --fail --silent http://127.0.0.1:8222/healthz | grep -F '\"status\":\"ok\"'")
  audit("${tomcatCorpus}", "curl --silent --max-time 5 http://127.0.0.1:8080/ >/dev/null")
  audit("${adminerCorpus}", "curl --fail --silent http://127.0.0.1:8080/ >/dev/null")
  audit("${phpmyadminCorpus}", "curl --fail --silent http://127.0.0.1:8080/ | grep -qi phpmyadmin")
  audit("${traefikCorpus}", "curl --fail --silent http://127.0.0.1:8080/ping | grep -Fx OK")

  audit("${caddyPack}", "curl --fail --silent http://127.0.0.1:8080/ | grep -F composix")
  audit("${devicesPack}", "systemctl is-active $(systemctl list-units --type=service --no-legend 'cix-run-*.service' | awk 'NF {print $1}' | tail -1)")
  audit("${listenfdsPack}", "curl --fail --silent http://127.0.0.1:18081/ | grep -Fx 'LISTEN_FDS=1; no socket() authority'", "http=127.0.0.1:18081")
  audit("${nginxPack}", "curl --fail --silent http://127.0.0.1:8080/ | grep -F composix")
  audit("${nodePack}", "curl --fail --silent http://127.0.0.1:8081/ | grep -Fx 'node JIT is enabled'")

  audit("${caddyCorpus}", "curl --fail --silent http://127.0.0.1:8080/ | grep -Fx Caddy")

  machine.succeed("install -m0600 /dev/null /run/closedroot-mastodon-secret")
  machine.succeed("printf 'mastodon-closedroot-password\\n' > /run/closedroot-mastodon-secret")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonPostgres}) closedroot-mastodon-postgres:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonRedis}) closedroot-mastodon-redis:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonWeb}) closedroot-mastodon-web:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonSidekiq}) closedroot-mastodon-sidekiq:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonStreaming}) closedroot-mastodon-streaming:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${mastodonCleanup}) closedroot-mastodon-cleanup:v1")
  machine.succeed("cp ${mastodonCompose} /tmp/scenario/mastodon.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix compose check /tmp/scenario/mastodon.json")

  started = time.time()
  machine.succeed("timeout 180 env CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/mastodon.json --closed-root --update-lock='*'")
  assert time.time() - started >= 2.5
  services = ["postgres", "redis", "web", "sidekiq", "streaming"]
  machine.succeed("systemctl is-active " + " ".join("cix-closedroot-mastodon-" + member + ".service" for member in services))
  machine.succeed("systemctl is-active cix-closedroot-mastodon-cleanup.timer")
  roots = []
  for member in services + ["cleanup"]:
      roots.append(assert_closed_unit("cix-closedroot-mastodon-" + member + ".service", member == "postgres"))

  machine.succeed("systemctl cat cix-closedroot-mastodon-web.service | grep -F 'LoadCredential=db-password:/run/closedroot-mastodon-secret'")
  machine.succeed("systemctl show cix-closedroot-mastodon-web.service --property=After --value | grep -F cix-closedroot-mastodon-postgres.service")
  machine.succeed("systemctl show cix-closedroot-mastodon-web.service --property=After --value | grep -F cix-closedroot-mastodon-redis.service")
  machine.succeed("curl --fail --silent http://127.0.0.1:33000/health | grep -Fx OK")
  machine.succeed("curl --fail --silent http://127.0.0.1:34000/api/v1/streaming/health | grep -Fx OK")
  shared = "/var/lib/cix-compose/closedroot-mastodon/shared/public-system"
  machine.succeed("grep -Fx credential-source=CREDENTIALS_DIRECTORY " + shared + "/web-started")
  machine.succeed("grep -Fx 'postgres=ok redis=ok' " + shared + "/web-ready")
  machine.succeed("grep -Fx 'postgres=ok redis=ok' " + shared + "/sidekiq-ready")
  machine.succeed("systemctl show cix-closedroot-mastodon-web.service --property=WatchdogUSec --value | grep -Fx 6s")
  machine.succeed("systemctl show cix-closedroot-mastodon-sidekiq.service --property=WatchdogUSec --value | grep -Fx 6s")
  time.sleep(7)
  machine.succeed("systemctl is-active cix-closedroot-mastodon-web.service cix-closedroot-mastodon-sidekiq.service")
  machine.wait_until_succeeds("journalctl --no-pager CIX_COMPOSITE=closedroot-mastodon CIX_SERVICE=cleanup | grep -F 'mastodon cleanup fired'", timeout=30)
  web_logs = machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix logs closedroot-mastodon/web -n 50")
  assert "mastodon web ready" in web_logs
  assert "mastodon sidekiq worker ready" not in web_logs

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down closedroot-mastodon --purge --yes")
  machine.succeed("test ! -e " + shared)
  for root in roots:
      machine.succeed("test ! -e " + root)
''
