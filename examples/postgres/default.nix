# PostgreSQL dogfood: initialize persistent state on first run, then serve it.
# Build: nix-build examples/postgres -o result-postgres
{ pkgs ? import <nixpkgs> { } }:

let
  start = pkgs.writeShellScript "postgres-cix-start" ''
    set -eu

    state_dir=/var/lib/postgresql
    data_dir="$state_dir/data"
    init_dir="$state_dir/.init"
    port="$1"
    uid="$(${pkgs.coreutils}/bin/id -u)"
    gid="$(${pkgs.coreutils}/bin/id -g)"

    printf 'cix:x:%s:%s:PostgreSQL:%s:/noshell\n' \
      "$uid" "$gid" "$state_dir" > "$state_dir/passwd"
    printf 'cix:x:%s:\n' "$gid" > "$state_dir/group"
    export NSS_WRAPPER_PASSWD="$state_dir/passwd"
    export NSS_WRAPPER_GROUP="$state_dir/group"
    export LD_PRELOAD="${pkgs.nss_wrapper}/lib/libnss_wrapper.so"

    if [ ! -s "$data_dir/PG_VERSION" ]; then
      ${pkgs.coreutils}/bin/rm -rf "$init_dir"
      ${pkgs.coreutils}/bin/mkdir -p "$init_dir"
      LANG=C LC_ALL=C ${pkgs.postgresql}/bin/initdb \
        --pgdata="$init_dir" \
        --username=cix \
        --auth-local=trust \
        --auth-host=trust \
        --encoding=UTF8 \
        --no-locale
      ${pkgs.coreutils}/bin/mv "$init_dir" "$data_dir"
    fi

    exec ${pkgs.postgresql}/bin/postgres \
      -D "$data_dir" \
      -p "$port" \
      -h 127.0.0.1 \
      -k "$state_dir"
  '';
in
pkgs.runCommand "postgres-cix" { } ''
  mkdir -p $out/bin
  ln -s ${start} $out/bin/start
  ln -s ${pkgs.postgresql}/bin/psql $out/bin/psql
  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 1,
    "services": {
      "postgres": {
        "exec": ["bin/start", "$PORT"],
        "env": { "PORT": { "type": "port", "default": 5432 } },
        "ports": { "postgres": { "env": "PORT", "protocol": "tcp" } },
        "dirs": { "state": ["/var/lib/postgresql"] }
      }
    }
  }
  EOF
''
