{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
in
scenario.node ''
  # CIP-83 keeps the native observability substrate visible: cix is only a projection.
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.db}) scenario-db:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.api "observe"}) scenario-api:v1")
  machine.succeed("cp ${scenario.composeFile "observe" "127.0.0.1:18085" "scenario-api:v1" null} /tmp/scenario/compose.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18085/ | grep -Fx observe:PONG")
  machine.succeed("journalctl --no-pager -u cix-observe-api.service | grep -F 'api-line observe'")
  machine.succeed("journalctl --no-pager CIX_COMPOSITE=observe | grep -F 'api-line observe'")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix logs observe/api --since yesterday | grep -F 'api-line observe'")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix ps | grep -F RESULT")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix stats | grep -F observe")
  machine.succeed("! journalctl --no-pager -u cix-observe-api.service | grep -F 'db-line ready'")
  machine.succeed("systemctl status cix-observe.slice | grep -F cix-observe-api.service")
  machine.succeed("test -d /sys/fs/cgroup/cix.slice/cix-observe.slice")
  machine.succeed("systemd-cgtop --batch --iterations=1 | grep -F cix-observe")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down observe")
''
