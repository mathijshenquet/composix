# PostgreSQL dogfood: initialize persistent state on first run, then serve it.
# Build: nix-build examples/postgres -o result-postgres
{ pkgs ? import <nixpkgs> { } }:

pkgs.runCommand "postgres-cix" { } ''
  mkdir -p $out/opt/postgres

  cat > $out/opt/postgres/runtime-env.sh <<'EOF'
  state_dir=/var/lib/postgresql
  data_dir="$state_dir/data"
  init_dir="$state_dir/.init"
  uid="$(id -u)"
  gid="$(id -g)"

  printf 'cix:x:%s:%s:PostgreSQL:%s:/noshell\n' \
    "$uid" "$gid" "$state_dir" > "$state_dir/passwd"
  printf 'cix:x:%s:\n' "$gid" > "$state_dir/group"
  export NSS_WRAPPER_PASSWD="$state_dir/passwd"
  export NSS_WRAPPER_GROUP="$state_dir/group"
  export LD_PRELOAD=/opt/postgres/libnss_wrapper.so
  EOF
  ln -s ${pkgs.nss_wrapper}/lib/libnss_wrapper.so $out/opt/postgres/libnss_wrapper.so

  cat > $out/opt/postgres/setup <<'EOF'
  #!${pkgs.runtimeShell}
  set -eu
  . /opt/postgres/runtime-env.sh

  if [ ! -s "$data_dir/PG_VERSION" ]; then
    rm -rf "$init_dir"
    mkdir -p "$init_dir"
    LANG=C LC_ALL=C initdb \
      --pgdata="$init_dir" \
      --username=cix \
      --auth-local=trust \
      --auth-host=trust \
      --encoding=UTF8 \
      --no-locale
    mv "$init_dir" "$data_dir"
  fi
  EOF

  cat > $out/opt/postgres/start <<'EOF'
  #!${pkgs.runtimeShell}
  set -eu
  . /opt/postgres/runtime-env.sh

  exec postgres \
    -D "$data_dir" \
    -p "$1" \
    -h 127.0.0.1 \
    -k /run/postgresql
  EOF

  chmod +x $out/opt/postgres/setup $out/opt/postgres/start
  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 2,
    "services": {
      "postgres": {
        "setup": ["${pkgs.bash}/bin/sh", "/opt/postgres/setup"],
        "exec": ["${pkgs.bash}/bin/sh", "/opt/postgres/start", "$PORT"],
        "env": {
          "PATH": { "default": "${pkgs.postgresql}/bin:${pkgs.coreutils}/bin" },
          "PORT": { "default": "5432" }
        },
        "mounts": ["/opt/postgres"],
        "ports": { "postgres": { "env": "PORT", "protocol": "tcp" } },
        "dirs": {
          "state": ["/var/lib/postgresql"],
          "run": ["/run/postgresql"]
        }
      }
    }
  }
  EOF
''
