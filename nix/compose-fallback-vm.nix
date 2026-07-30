{ pkgs, cix }:

let
  producer = pkgs.runCommand "compose-fallback-producer" { } ''
    mkdir -p $out/bin
    cat > $out/bin/producer <<'EOF'
    #!${pkgs.runtimeShell}
    set -eu
    ${pkgs.coreutils}/bin/touch /var/lib/fallback-producer/state-ready
    ${pkgs.coreutils}/bin/touch /run/fallback-edge/edge-ready
    exec ${pkgs.coreutils}/bin/sleep infinity
    EOF
    chmod +x $out/bin/producer
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 4,
      "exec": ["bin/producer"],
      "dirs": {
        "state": ["/var/lib/fallback-producer"],
        "run": ["/run/fallback-edge"]
      }
    }
    EOF
  '';

  consumer = pkgs.runCommand "compose-fallback-consumer" { } ''
    mkdir -p $out/bin
    cat > $out/bin/consumer <<'EOF'
    #!${pkgs.runtimeShell}
    exec ${pkgs.coreutils}/bin/sleep infinity
    EOF
    chmod +x $out/bin/consumer
    cat > $out/cix-manifest.json <<'EOF'
    {
      "cixManifest": 4,
      "exec": ["bin/consumer"]
    }
    EOF
  '';

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
    machine.succeed("rm /etc/systemd/system && mkdir /etc/systemd/system")
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
    machine.wait_until_succeeds("test -f /var/lib/fallback-producer/state-ready")
    machine.wait_until_succeeds("test -f /run/cix-fallback-edge-shared/edge-ready")
    machine.succeed("cix down fallback")
  '';
}
