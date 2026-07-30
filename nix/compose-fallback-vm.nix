{ pkgs, cix }:

let
  producer = pkgs.writeTextDir "cix-manifest.json" (builtins.toJSON {
    cixManifest = 4;
    exec = [ "${pkgs.coreutils}/bin/sleep" "infinity" ];
    dirs = {
      state = [ "/var/lib/fallback-producer" ];
      run = [ "/run/fallback-edge" ];
    };
  });

  consumer = pkgs.writeTextDir "cix-manifest.json" (builtins.toJSON {
    cixManifest = 4;
    exec = [ "${pkgs.coreutils}/bin/sleep" "infinity" ];
  });

  compose = pkgs.writeText "compose-fallback.json" (builtins.toJSON {
    composeVersion = 1;
    name = "fallback";
    services = {
      producer.item = producer;
      consumer.item = consumer;
    };
    edges.shared = {
      producer = {
        service = "producer";
        path = "/run/fallback-edge";
      };
      consumers.consumer = { };
    };
  });

  lock = pkgs.writeText "compose-fallback.lock" (builtins.toJSON {
    services = {
      producer = {
        ref = toString producer;
        storePath = toString producer;
        narHash = "sha256-compose-fallback-producer";
      };
      consumer = {
        ref = toString consumer;
        storePath = toString consumer;
        narHash = "sha256-compose-fallback-consumer";
      };
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "compose-fallback";

  nodes.machine = { ... }: {
    environment.systemPackages = [ cix pkgs.jq ];
    nix.settings.experimental-features = [ "nix-command" ];

    networking.useDHCP = false;
    networking.interfaces.eth0.useDHCP = false;
    system.stateVersion = "24.11";
  };

  testScript = ''
    start_all()
    machine.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
    machine.succeed("mkdir /run/cix-system-units && cp -a /etc/systemd/system/. /run/cix-system-units/ && rm /etc/systemd/system && mv /run/cix-system-units /etc/systemd/system")
    machine.succeed("mkdir -p /tmp/fallback && cp ${compose} /tmp/fallback/compose.json && cp ${lock} /tmp/fallback/cix.lock")
    status, warning = machine.execute("cix up /tmp/fallback/compose.json 2>&1")
    print(warning)
    assert status == 0

    assert "unit cix-fallback-producer.service" in warning
    assert "dropped PrivatePIDs=yes" in warning
    assert "systemd 261 failed the DynamicUser=yes + PrivatePIDs=yes + StateDirectory= realization probe" in warning
    assert "shares the host PID namespace (D36 degraded fallback)" in warning

    manifest = "/nix/var/nix/profiles/cix-compose-fallback/manifest.json"
    machine.succeed(
        "jq -e '.degradations == [{\"unit\":\"cix-fallback-producer.service\",\"property\":\"PrivatePIDs=yes\",\"reason\":\"systemd 261 failed the DynamicUser=yes + PrivatePIDs=yes + StateDirectory= realization probe\"}]' "
        + manifest
    )
    machine.succeed("! grep -q '^PrivatePIDs=' /etc/systemd/system/cix-fallback-producer.service")
    machine.succeed("grep -q '^PrivatePIDs=yes' /etc/systemd/system/cix-fallback-consumer.service")
    machine.wait_for_unit("cix-fallback.target")
    machine.wait_for_unit("cix-fallback-producer.service")
    machine.wait_for_unit("cix-fallback-consumer.service")
    machine.succeed("test -d /var/lib/private/cix-fallback-producer")
    machine.succeed("test -d /run/cix-fallback-edge-shared")
  '';
}
