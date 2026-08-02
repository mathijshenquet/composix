{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  shell = pkgs.runtimeShell;
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
    mkdir -p /var/lib/host /media
    touch /var/lib/host/host-state /media/host-media
    while true; do sleep 1; done
  '';
  private = item "scenario-dirs2-private" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"state":["/var/lib/private"],"cache":["/var/cache/private"]}}
  '' ''
    mkdir -p /var/lib/private /var/cache/private
    touch /var/lib/private/survives /var/cache/private/expendable
    while true; do sleep 1; done
  '';
  shared = side: item "scenario-dirs2-shared-${side}" ''
    {"cixManifest":0,"start":["bin/start"],"dirs":{"state":["/var/lib/shared"]}}
  '' ''
    mkdir -p /var/lib/shared
    touch /var/lib/shared/${side}
    while true; do sleep 1; done
  '';
  compose = pkgs.writeText "scenario-dirs2.json" ''
    {
      "composeVersion": 1,
      "name": "dirs2",
      "services": {
        "host": {
          "item": "scenario-dirs2-host:v1",
          "identity": "dirs2host",
          "dirs": {
            "/var/lib/host": {"host": "/tmp/dirs2/host-state"},
            "/media": {"host": "/tmp/dirs2/host-media"}
          }
        },
        "private": {"item": "scenario-dirs2-private:v1"},
        "left": {"item": "scenario-dirs2-left:v1", "dirs": {"/var/lib/shared": {"shared": "uploads"}}},
        "right": {"item": "scenario-dirs2-right:v1", "dirs": {"/var/lib/shared": {"shared": "uploads"}}}
      }
    }
  '';
  degradation = pkgs.writeText "scenario-dirs2-degrade.json" ''
    {
      "composeVersion": 1,
      "name": "dirs2-degrade",
      "services": {
        "private": {"item": "scenario-dirs2-private:v1", "dirs": {"/var/lib/private": {"as": "cache"}}}
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
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${shared "left"}) scenario-dirs2-left:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${shared "right"}) scenario-dirs2-right:v1")
  machine.succeed("cp ${compose} /tmp/scenario/dirs2.json")
  status, warning = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix compose check ${degradation} 2>&1")
  print(warning)
  assert status == 0
  assert "LOUD durability degradation" in warning
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/dirs2.json")
  machine.succeed("systemctl is-active cix-dirs2-host.service cix-dirs2-private.service cix-dirs2-left.service cix-dirs2-right.service")
  machine.succeed("test -f /tmp/dirs2/host-state/host-state /tmp/dirs2/host-media/host-media")
  machine.succeed("test -f /var/lib/cix-compose/dirs2/shared/uploads/left /var/lib/cix-compose/dirs2/shared/uploads/right")
  machine.succeed("test $(stat -c %a /var/lib/cix-compose/dirs2/shared/uploads) = 2770")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix clean dirs2 --what=cache")
  machine.succeed("test ! -e /var/cache/cix-dirs2-private/var/cache/private/expendable")
  status, refusal = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix clean dirs2 --what=state 2>&1")
  assert status != 0
  assert "refusing to clean STATEDIR" in refusal
  status, purge = machine.execute("CIX_STATE_DIR=/var/lib/cix-index cix down dirs2 --purge --yes 2>&1")
  print(purge)
  assert status == 0
  assert "/var/lib/cix-dirs2-private/var/lib/private" in purge
  assert "/var/lib/cix-compose/dirs2/shared/uploads" in purge
  machine.succeed("test -f /tmp/dirs2/host-state/host-state /tmp/dirs2/host-media/host-media")
  machine.succeed("test ! -e /var/lib/cix-dirs2-private/var/lib/private")
  machine.succeed("test ! -e /var/lib/cix-compose/dirs2/shared/uploads")
''
