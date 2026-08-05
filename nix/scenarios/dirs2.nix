{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  shell = pkgs.runtimeShell;
  python = pkgs.python3;
  item = name: manifest: body: pkgs.runCommand name { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/start" <<'SH'
    #!${shell}
    ${body}
    SH
    chmod 0755 "$out/bin/start"
    cat > "$out/cix-manifest.json" <<EOF
    ${manifest}
    EOF
  '';
  host = item "scenario-dirs2-host" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"state":["/var/lib/host"],"data":[{"path":"/media","ro":false}]}}
  '' ''
    ${pkgs.coreutils}/bin/mkdir -p /var/lib/host /media
    ${pkgs.coreutils}/bin/touch /var/lib/host/host-state /media/host-media
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
  '';
  private = item "scenario-dirs2-private" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"state":["/var/lib/private"],"cache":["/var/cache/private"]}}
  '' ''
    ${pkgs.coreutils}/bin/mkdir -p /var/lib/private /var/cache/private
    ${pkgs.coreutils}/bin/touch /var/lib/private/survives /var/cache/private/expendable
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
  '';
  nested = pkgs.runCommand "scenario-dirs2-nested" { } ''
    mkdir -p "$out/bin" "$out/var/www/images/uploads/logos"
    printf 'artifact\n' > "$out/var/www/immutable-marker"
    cat > "$out/bin/start" <<'SH'
    #!${shell}
    set -eu
    test "$(cat /var/www/immutable-marker)" = artifact
    touch /var/www/db/nested-state /var/www/images/uploads/logos/nested-logo
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
    SH
    chmod 0755 "$out/bin/start"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/start"],"mounts":["/var/www"],"dirs":{"state":["/var/www/db","/var/www/images/uploads/logos"]}}
    EOF
  '';
  shared = side: item "scenario-dirs2-shared-${side}" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"state":["/var/lib/shared"]}}
  '' ''
    ${pkgs.coreutils}/bin/mkdir -p /var/lib/shared
    ${pkgs.coreutils}/bin/touch /var/lib/shared/${side}
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
  '';
  config = item "scenario-dirs2-config" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"config":["/config/probe"]}}
  '' ''
    set -eu
    test "$CONFIGURATION_DIRECTORY" = /config/probe
    ${pkgs.coreutils}/bin/touch /config/probe/configdir-projected
    echo configdir-projected
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
  '';
  localhost = item "scenario-dirs2-localhost" ''
    {"cixManifest":0,"start":["bin/start"]}
  '' ''
    set -eu
    ${python}/bin/python3 -c 'import socket; assert "127.0.0.1" in {row[4][0] for row in socket.getaddrinfo("localhost", None)}'
    echo localhost-skeleton
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
  '';
  localhostOverride = pkgs.runCommand "scenario-dirs2-localhost-override" { } ''
    mkdir -p "$out/bin" "$out/etc"
    cat > "$out/bin/start" <<'SH'
    #!${shell}
    set -eu
    ${python}/bin/python3 -c 'import socket; assert "127.0.0.42" in {row[4][0] for row in socket.getaddrinfo("localhost", None)}'
    echo localhost-item-override
    while true; do ${pkgs.coreutils}/bin/sleep 1; done
    SH
    chmod 0755 "$out/bin/start"
    printf '127.0.0.42 item-wins localhost\n' > "$out/etc/hosts"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/start"],"mounts":["/etc/hosts"]}
    EOF
  '';
  compose = pkgs.writeText "scenario-dirs2.json" ''
    {
      "cixCompose": 1,
      "name": "dirs2",
      "children": {
        "host": {
          "item": "scenario-dirs2-host:v1",
          "identity": "dirs2host",
          "dirs": {
            "/var/lib/host": {"host": "/tmp/dirs2/host-state"},
            "/media": {"host": "/tmp/dirs2/host-media"}
          }
        },
        "private": {"item": "scenario-dirs2-private:v1"},
        "nested": {"item": "scenario-dirs2-nested:v1"},
        "left": {"item": "scenario-dirs2-left:v1", "dirs": {"/var/lib/shared": {"shared": "uploads"}}},
        "right": {"item": "scenario-dirs2-right:v1", "dirs": {"/var/lib/shared": {"shared": "uploads"}}}
      }
    }
  '';
  degradation = pkgs.writeText "scenario-dirs2-degrade.json" ''
    {
      "cixCompose": 1,
      "name": "dirs2-degrade",
      "children": {
        "private": {"item": "scenario-dirs2-private:v1", "dirs": {"/var/lib/private": {"as": "cache"}}}
      }
    }
  '';
  sealed = pkgs.writeText "scenario-dirs2-sealed.json" ''
    {
      "cixCompose": 1,
      "name": "dirs2-sealed",
      "children": {
        "config": {"item": "scenario-dirs2-config:v1"},
        "localhost": {"item": "scenario-dirs2-localhost:v1"},
        "override": {"item": "scenario-dirs2-localhost-override:v1"}
      }
    }
  '';
in
scenario.node ''
  machine.succeed("mkdir -p /tmp/dirs2/host-state /tmp/dirs2/host-media /tmp/scenario")
  machine.succeed("printf 'u dirs2host - - -\\n' > /tmp/dirs2host.conf; systemd-sysusers /tmp/dirs2host.conf")
  machine.succeed("chown -R dirs2host:dirs2host /tmp/dirs2/host-state /tmp/dirs2/host-media")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${host}) scenario-dirs2-host:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${private}) scenario-dirs2-private:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${nested}) scenario-dirs2-nested:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${shared "left"}) scenario-dirs2-left:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${shared "right"}) scenario-dirs2-right:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${config}) scenario-dirs2-config:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${localhost}) scenario-dirs2-localhost:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${localhostOverride}) scenario-dirs2-localhost-override:v1")
  machine.succeed("cp ${compose} /tmp/scenario/dirs2.json")
  machine.succeed("cp ${sealed} /tmp/scenario/dirs2-sealed.json")
  status, warning = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix compose check ${degradation} 2>&1")
  print(warning)
  assert status == 0
  assert "LOUD durability degradation" in warning
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/dirs2.json")
  machine.succeed("systemctl is-active cix-dirs2-host.service cix-dirs2-private.service cix-dirs2-nested.service cix-dirs2-left.service cix-dirs2-right.service")
  machine.wait_until_succeeds("test -f /tmp/dirs2/host-state/host-state", timeout=60)
  machine.wait_until_succeeds("test -f /tmp/dirs2/host-media/host-media", timeout=60)
  machine.wait_until_succeeds("test -f /var/lib/cix-dirs2-nested/var/www/db/nested-state", timeout=60)
  machine.wait_until_succeeds("test -f /var/lib/cix-dirs2-nested/var/www/images/uploads/logos/nested-logo", timeout=60)
  machine.succeed("systemctl show cix-dirs2-nested.service --property=BindReadOnlyPaths --value | grep -F :/var/www")
  machine.wait_until_succeeds("test -f /var/lib/cix-compose/dirs2/shared/uploads/left", timeout=60)
  machine.wait_until_succeeds("test -f /var/lib/cix-compose/dirs2/shared/uploads/right", timeout=60)
  machine.succeed("systemctl is-active cix-dirs2-host.service cix-dirs2-private.service cix-dirs2-nested.service cix-dirs2-left.service cix-dirs2-right.service")
  for unit in ["cix-dirs2-host", "cix-dirs2-private", "cix-dirs2-nested", "cix-dirs2-left", "cix-dirs2-right"]:
      machine.succeed("test $(systemctl show " + unit + ".service -p NRestarts --value) = 0")
  machine.succeed("test $(stat -c %a /var/lib/cix-compose/dirs2/shared/uploads) = 2770")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/dirs2-sealed.json --closed-root")
  machine.succeed("systemctl is-active cix-dirs2-sealed-config.service cix-dirs2-sealed-localhost.service cix-dirs2-sealed-override.service")
  machine.wait_until_succeeds("journalctl --no-pager -u cix-dirs2-sealed-config.service | grep -F configdir-projected", timeout=60)
  machine.wait_until_succeeds("journalctl --no-pager -u cix-dirs2-sealed-localhost.service | grep -F localhost-skeleton", timeout=60)
  machine.wait_until_succeeds("journalctl --no-pager -u cix-dirs2-sealed-override.service | grep -F localhost-item-override", timeout=60)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix clean dirs2 --what=cache")
  machine.succeed("test $(systemctl show cix-dirs2-private.service -p ActiveState --value) = inactive")
  machine.succeed("systemctl is-active cix-dirs2-host.service cix-dirs2-nested.service cix-dirs2-left.service cix-dirs2-right.service")
  machine.succeed("test ! -e /var/cache/cix-dirs2-private/var/cache/private/expendable")
  status, refusal = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix clean dirs2 --what=state 2>&1")
  assert status != 0
  assert "refusing to clean STATEDIR" in refusal
  status, purge = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix down dirs2 --purge --yes 2>&1")
  print(purge)
  assert status == 0
  assert "/var/lib/cix-dirs2-private/var/lib/private" in purge
  assert "/var/lib/cix-compose/dirs2/shared/uploads" in purge
  machine.succeed("test -f /tmp/dirs2/host-state/host-state")
  machine.succeed("test -f /tmp/dirs2/host-media/host-media")
  machine.succeed("test ! -e /var/lib/cix-dirs2-private/var/lib/private")
  machine.succeed("test ! -e /var/lib/cix-compose/dirs2/shared/uploads")
''
