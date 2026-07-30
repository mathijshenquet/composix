{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
in
scenario.node ''
  # D44 FRONTIER: --update <edge> selective repin on nested composites
  # D44 root-side tracking is an explicit compose policy, independent of the tag's spelling.
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.db}) scenario-db:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.api "v1"}) scenario-api:track")
  machine.succeed("cp ${scenario.trackedComposeFile "repin" "127.0.0.1:18083" "scenario-api:track" null} /tmp/scenario/compose.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18083/ | grep -Fx v1:PONG")
  old_generation = machine.succeed("readlink -f /nix/var/nix/profiles/cix-compose-repin").strip()
  old_api = machine.succeed("jq -r .services.api.storePath " + old_generation + "/manifest.json").strip()
  db_before = machine.succeed("systemctl show cix-repin-db.service -p ActiveEnterTimestampMonotonic --value").strip()
  api_before = machine.succeed("systemctl show cix-repin-api.service -p ActiveEnterTimestampMonotonic --value").strip()
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.api "v2"}) scenario-api:track")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  new_generation = machine.succeed("readlink -f /nix/var/nix/profiles/cix-compose-repin").strip()
  new_api = machine.succeed("jq -r .services.api.storePath " + new_generation + "/manifest.json").strip()
  assert old_generation != new_generation
  assert old_api != new_api
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18083/ | grep -Fx v2:PONG")
  machine.succeed("test $(systemctl show cix-repin-db.service -p ActiveEnterTimestampMonotonic --value) = " + db_before)
  machine.succeed("test $(systemctl show cix-repin-api.service -p ActiveEnterTimestampMonotonic --value) != " + api_before)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix rollback repin")
  machine.succeed("test $(jq -r .services.api.storePath $(readlink -f /nix/var/nix/profiles/cix-compose-repin)/manifest.json) = " + old_api)
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18083/ | grep -Fx v1:PONG")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down repin")
''
