# PostgreSQL dogfood: initialize persistent state on first run, then serve it.
# Build: nix-build examples/postgres -o result-postgres
{ pkgs ? import <nixpkgs> { } }:

pkgs.runCommand "postgres-cix" { } ''
  mkdir -p $out/bin $out/lib

  cat > $out/lib/runtime-env.sh <<'EOF'
  state_dir=/var/lib/postgresql
  data_dir="$state_dir/data"
  init_dir="$state_dir/.init"
  uid="$(${pkgs.coreutils}/bin/id -u)"
  gid="$(${pkgs.coreutils}/bin/id -g)"

  printf 'cix:x:%s:%s:PostgreSQL:%s:/noshell\n' \
    "$uid" "$gid" "$state_dir" > "$state_dir/passwd"
  printf 'cix:x:%s:\n' "$gid" > "$state_dir/group"
  export NSS_WRAPPER_PASSWD="$state_dir/passwd"
  export NSS_WRAPPER_GROUP="$state_dir/group"
  export LD_PRELOAD="${pkgs.nss_wrapper}/lib/libnss_wrapper.so"
  EOF

  cat > $out/bin/setup <<EOF
  #!${pkgs.runtimeShell}
  set -eu
  . "$out/lib/runtime-env.sh"

  if [ ! -s "\$data_dir/PG_VERSION" ]; then
    ${pkgs.coreutils}/bin/rm -rf "\$init_dir"
    ${pkgs.coreutils}/bin/mkdir -p "\$init_dir"
    LANG=C LC_ALL=C ${pkgs.postgresql}/bin/initdb \
      --pgdata="\$init_dir" \
      --username=cix \
      --auth-local=trust \
      --auth-host=trust \
      --encoding=UTF8 \
      --no-locale
    ${pkgs.coreutils}/bin/mv "\$init_dir" "\$data_dir"
  fi
  EOF

  cat > $out/bin/start <<EOF
  #!${pkgs.runtimeShell}
  set -eu
  . "$out/lib/runtime-env.sh"

  exec ${pkgs.postgresql}/bin/postgres \
    -D "\$data_dir" \
    -p "\$1" \
    -h 127.0.0.1 \
    -k /run/postgresql
  EOF

  chmod +x $out/bin/setup $out/bin/start
  ln -s ${pkgs.postgresql}/bin/psql $out/bin/psql
  cat > $out/cix-spec.json <<'EOF'
  {
    "cixSpec": 2,
    "services": {
      "postgres": {
        "setup": ["bin/setup"],
        "exec": ["bin/start", "$PORT"],
        "env": { "PORT": { "default": "5432" } },
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
