{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
in
scenario.node ''
  # D43 FRONTIER (flip when pod-ness lands): identical internal ports without any bind conflict once both claim network: pod
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.db}) scenario-db:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.api "side"}) scenario-api:v1")
  machine.succeed("cp ${scenario.composeFile "alpha" "127.0.0.1:18081" "scenario-api:v1" null} /tmp/scenario/alpha.json")
  machine.succeed("cp ${scenario.composeFile "beta" "127.0.0.1:18082" "scenario-api:v1" null} /tmp/scenario/beta.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/alpha.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/beta.json")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18081/ | grep -Fx side:PONG")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18082/ | grep -Fx side:PONG")
  machine.succeed("test $(systemctl show cix-alpha.slice -p ControlGroup --value) = /cix.slice/cix-alpha.slice")
  machine.succeed("test $(systemctl show cix-beta.slice -p ControlGroup --value) = /cix.slice/cix-beta.slice")
  machine.succeed("test -d /run/cix-alpha-edge-database; test -d /run/cix-beta-edge-database")
  machine.succeed("test ! -e /run/cix-alpha-edge-database/.same-as-beta")
  machine.succeed("cp ${scenario.composeFile "collision" "127.0.0.1:18081" "scenario-api:v1" null} /tmp/scenario/collision.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/collision.json")
  machine.succeed("systemctl is-failed cix-collision-api-http.socket")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down alpha; CIX_STATE_DIR=/var/lib/cix-index cix down beta")
  machine.succeed("systemctl reset-failed 'cix-*' || true")
''
