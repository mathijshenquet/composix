{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
in
scenario.node ''
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.db}) scenario-db:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${scenario.api "v1"}) scenario-api:track")
  machine.succeed("cp ${scenario.composeFile "lifecycle" "127.0.0.1:18080" "scenario-api:track" "v1"} /tmp/scenario/compose.json")
  systemd_version = machine.succeed("systemctl --version | head -1 | awk '{ print $2 }'").strip()
  status, warning = machine.execute("timeout 60 env CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json 2>&1")
  print(warning)
  assert status == 0
  degradation_reason = "systemd " + systemd_version + " failed the DynamicUser=yes + PrivatePIDs=yes + StateDirectory= realization probe"
  assert "unit cix-lifecycle-api.service" in warning
  assert "dropped PrivatePIDs=yes" in warning
  assert degradation_reason in warning
  assert "shares the host PID namespace (D36 degraded fallback)" in warning
  machine.succeed(
      "jq -e --arg reason \"" + degradation_reason + "\" "
      + "'.degradations == [{\"unit\":\"cix-lifecycle-api.service\",\"property\":\"PrivatePIDs=yes\",\"reason\":$reason}]' "
      + "/nix/var/nix/profiles/cix-compose-lifecycle/manifest.json"
  )
  machine.succeed("systemctl is-active cix-lifecycle-api.service cix-lifecycle-db.service cix-lifecycle-api-http.socket")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18080/ | grep -Fx v1:PONG")
  machine.succeed("test -f /var/lib/private/cix-lifecycle-api/sentinel")
  generation_one = machine.succeed("readlink -f /nix/var/nix/profiles/cix-compose-lifecycle").strip()
  db_before = machine.succeed("systemctl show cix-lifecycle-db.service -p ActiveEnterTimestampMonotonic --value").strip()
  api_before = machine.succeed("systemctl show cix-lifecycle-api.service -p ActiveEnterTimestampMonotonic --value").strip()
  machine.succeed("sed -i 's/\"MESSAGE\": \"v1\"/\"MESSAGE\": \"v2\"/' /tmp/scenario/compose.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/compose.json")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18080/ | grep -Fx v2:PONG")
  machine.succeed("test $(systemctl show cix-lifecycle-db.service -p ActiveEnterTimestampMonotonic --value) = " + db_before)
  machine.succeed("test $(systemctl show cix-lifecycle-api.service -p ActiveEnterTimestampMonotonic --value) != " + api_before)
  machine.succeed("test -f /var/lib/private/cix-lifecycle-api/sentinel")
  machine.succeed("test $(readlink -f /nix/var/nix/profiles/cix-compose-lifecycle) != " + generation_one)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix rollback lifecycle")
  machine.wait_until_succeeds("curl --max-time 5 --fail --silent http://127.0.0.1:18080/ | grep -Fx v1:PONG")
  machine.succeed("test $(readlink -f /nix/var/nix/profiles/cix-compose-lifecycle) = " + generation_one)
  machine.succeed("test -f /var/lib/private/cix-lifecycle-api/sentinel")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down lifecycle")
  machine.succeed("systemctl reset-failed 'cix-lifecycle*' || true")
  machine.succeed("test -z \"$(systemctl list-units --all --no-legend 'cix-lifecycle*' | awk 'NF { print $1 }')\"")
''
