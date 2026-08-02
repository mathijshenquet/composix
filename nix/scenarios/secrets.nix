{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  consumer = pkgs.runCommand "scenario-secrets-consumer" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/consumer" <<'SH'
    #!${pkgs.bash}/bin/sh
    case "$( ${pkgs.coreutils}/bin/cat "$DB_PASSWORD_FILE")" in first|second) ;; *) exit 1 ;; esac
    ${pkgs.coreutils}/bin/printf 'CIX_SECRET_PATH=%s\n' "$DB_PASSWORD_FILE" >&2
    trap 'exit 0' TERM
    while :; do ${pkgs.coreutils}/bin/sleep 1; done
    SH
    chmod 0755 "$out/bin/consumer"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/consumer"],"secrets":{"db-password":{"as":"DB_PASSWORD_FILE"}}}
    EOF
  '';
  helper = pkgs.runCommand "scenario-secrets-helper" { } ''
    mkdir -p "$out/bin"
    printf '#!%s\ntrap "exit 0" TERM\nwhile :; do %s/bin/sleep 1; done\n' ${pkgs.bash}/bin/sh ${pkgs.coreutils} > "$out/bin/helper"
    chmod 0755 "$out/bin/helper"
    echo '{"cixManifest":0,"start":["bin/helper"]}' > "$out/cix-manifest.json"
  '';
  compose = pkgs.writeText "scenario-secrets.json" ''
    {"cixCompose":1,"name":"secrets","children":{"consumer":{"item":"scenario-secrets-consumer:v1"},"helper":{"item":"scenario-secrets-helper:v1"}},"secrets":{"db-password":{"file":"/run/cix-test-secret"}}}
  '';
  runCompose = pkgs.writeText "scenario-secrets-run.json" ''
    {"cixCompose":1,"name":"secretsrun","children":{"consumer":{"item":"scenario-secrets-consumer:v1"}},"secrets":{"db-password":{"file":"/run/cix-test-secret"}}}
  '';
  strayCompose = pkgs.writeText "scenario-secrets-stray.json" ''
    {"cixCompose":1,"name":"secrets","children":{"consumer":{"item":"scenario-secrets-consumer:v1"}},"secrets":{"db-password":{"file":"/run/cix-test-secret"},"stray":{"file":"/run/cix-test-secret"}}}
  '';
in
scenario.node ''
  machine.succeed("install -m 0600 -o root -g root /dev/null /run/cix-test-secret")
  machine.succeed("printf first > /run/cix-test-secret")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${consumer}) scenario-secrets-consumer:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${helper}) scenario-secrets-helper:v1")
  machine.succeed("cp ${compose} /tmp/scenario/secrets.json; cp ${runCompose} /tmp/scenario/secrets-run.json; cp ${strayCompose} /tmp/scenario/secrets-stray.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix compose check /tmp/scenario/secrets.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix compose check /tmp/scenario/secrets-stray.json 2>&1 | grep -F LOUD")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/secrets.json")
  machine.succeed("systemctl is-active cix-secrets-consumer.service cix-secrets-helper.service")
  machine.wait_until_succeeds("journalctl -u cix-secrets-consumer.service | grep -F CIX_SECRET_PATH=/run/credentials/cix-secrets-consumer.service/db-password")
  consumer_before = machine.succeed("systemctl show cix-secrets-consumer.service -p InvocationID --value").strip()
  helper_before = machine.succeed("systemctl show cix-secrets-helper.service -p InvocationID --value").strip()
  machine.succeed("printf second > /run/cix-test-secret")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/secrets.json")
  machine.wait_until_succeeds("test \"$(systemctl show cix-secrets-consumer.service -p InvocationID --value)\" != " + consumer_before)
  machine.succeed("test \"$(systemctl show cix-secrets-helper.service -p InvocationID --value)\" = " + helper_before)
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix run --compose /tmp/scenario/secrets-run.json")
  machine.succeed("systemctl is-active cix-secretsrun-consumer.service")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down secretsrun; CIX_STATE_DIR=/var/lib/cix-index cix down secrets")
''
