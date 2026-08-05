{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  item = pkgs.runCommand "scenario-stopdispo-item" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/stopdispo" <<'SH'
    #!${pkgs.runtimeShell}
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/stopdispo"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/stopdispo"],"stopSignal":"SIGQUIT"}
    EOF
  '';
  compose = pkgs.writeText "scenario-stopdispo.json" ''
    {
      "cixCompose": 1,
      "name": "stopdispo",
      "children": {
        "service": {
          "item": "scenario-stopdispo:v1",
          "stopTimeout": "3s"
        }
      }
    }
  '';
in
scenario.node ''
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${item}) scenario-stopdispo:v1")
  machine.succeed("cp ${compose} /tmp/scenario/compose.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  machine.succeed("systemctl cat cix-stopdispo-service.service | grep -Fx 'KillSignal=SIGQUIT'")
  machine.succeed("test \"$(systemctl show cix-stopdispo-service.service --property=TimeoutStopUSec --value)\" = 3s")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down stopdispo")
''
