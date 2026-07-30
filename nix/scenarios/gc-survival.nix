{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
in
scenario.node ''
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag ${scenario.db} scenario-db:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag ${scenario.api "live"} scenario-api:track")
  machine.succeed("printf '%s' '${scenario.compose "gc" "127.0.0.1:18084" "scenario-api:track" null}' > /tmp/scenario/compose.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:18084/ | grep -Fx live:PONG")
  active = machine.succeed("readlink -f /nix/var/nix/profiles/cix-compose-gc").strip()
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag ${scenario.api "new"} scenario-api:track")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  current_api = machine.succeed("jq -r .services.api.storePath $(readlink -f /nix/var/nix/profiles/cix-compose-gc)/manifest.json").strip()
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix index history scenario-api > /tmp/scenario/history-before")
  machine.succeed("nix-collect-garbage")
  machine.succeed("test -e $(readlink -f /nix/var/nix/profiles/cix-compose-gc); test -e " + current_api)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix ls | grep -Fx scenario-api:track")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix inspect scenario-api:track | grep -F " + current_api)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix index history scenario-api > /tmp/scenario/history-after")
  machine.succeed("test -s /tmp/scenario/history-after")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down gc")
''
