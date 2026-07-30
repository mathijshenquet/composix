{ pkgs, cix }:

let
  probeService = properties: {
    wantedBy = [ ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${pkgs.coreutils}/bin/true";
    } // properties;
  };
in
pkgs.testers.runNixOSTest {
  name = "compose-fallback";

  nodes.machine = { ... }: {
    environment.systemPackages = [ cix ];

    systemd.services = {
      compose-probe-edge = {
        wantedBy = [ ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${pkgs.coreutils}/bin/true";
          RuntimeDirectory = "compose-probe-edge";
          RuntimeDirectoryMode = "0770";
        };
      };
      compose-probe-full = probeService {
        DynamicUser = true;
        PrivatePIDs = true;
        StateDirectory = "compose-probe-full";
        BindPaths = "/run/compose-probe-edge:/run/compose-probe-app:rbind";
        ProtectSystem = "strict";
        PrivateTmp = true;
      };
      compose-probe-no-private-pids = probeService {
        DynamicUser = true;
        StateDirectory = "compose-probe-no-private-pids";
        BindPaths = "/run/compose-probe-edge:/run/compose-probe-app:rbind";
        ProtectSystem = "strict";
        PrivateTmp = true;
      };
      compose-probe-no-dynamic-user = probeService {
        PrivatePIDs = true;
        StateDirectory = "compose-probe-no-dynamic-user";
        BindPaths = "/run/compose-probe-edge:/run/compose-probe-app:rbind";
        ProtectSystem = "strict";
        PrivateTmp = true;
      };
      compose-probe-no-state = probeService {
        DynamicUser = true;
        PrivatePIDs = true;
        BindPaths = "/run/compose-probe-edge:/run/compose-probe-app:rbind";
        ProtectSystem = "strict";
        PrivateTmp = true;
      };
      compose-probe-minimal = probeService {
        DynamicUser = true;
        PrivatePIDs = true;
        StateDirectory = "compose-probe-minimal";
      };
      compose-probe-runtime = probeService {
        DynamicUser = true;
        PrivatePIDs = true;
        RuntimeDirectory = "compose-probe-runtime";
      };
    };

    networking.useDHCP = false;
    networking.interfaces.eth0.useDHCP = false;
    system.stateVersion = "24.11";
  };

  testScript = ''
    start_all()
    machine.succeed("systemctl start compose-probe-edge.service")
    machine.fail("systemctl start compose-probe-full.service")
    machine.succeed("systemctl show compose-probe-full.service --property=Result --value | grep -Fx exit-code")
    machine.succeed("journalctl -u compose-probe-full.service --no-pager | grep -F 'Failed to allocate user namespace'")
    machine.succeed("systemctl start compose-probe-no-private-pids.service")
    machine.succeed("systemctl start compose-probe-no-dynamic-user.service")
    machine.succeed("systemctl start compose-probe-no-state.service")
    machine.fail("systemctl start compose-probe-minimal.service")
    machine.succeed("journalctl -u compose-probe-minimal.service --no-pager | grep -F 'Failed to allocate user namespace'")
    machine.succeed("systemctl start compose-probe-runtime.service")
  '';
}
