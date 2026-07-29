{ pkgs ? import <nixpkgs> { } }:

pkgs.runCommand "dstyle-postgres-unix" { } ''
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
    -c listen_addresses= \
    -c unix_socket_directories=/run/postgresql \
    -c unix_socket_permissions=0777
  EOF

  chmod +x $out/bin/setup $out/bin/start
  ln -s ${pkgs.postgresql}/bin/psql $out/bin/psql

  cat > $out/cix-manifest.json <<'EOF'
  {
    "cixManifest": 2,
    "services": {
      "postgres": {
        "setup": ["bin/setup"],
        "exec": ["bin/start"],
        "dirs": {
          "state": ["/var/lib/postgresql"],
          "run": ["/run/postgresql"]
        }
      }
    }
  }
  EOF
''

