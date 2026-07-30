{ pkgs, revertedSystemd }:

pkgs.testers.runNixOSTest {
  name = "sdbisect-revert";

  nodes = {
    stock = { pkgs, ... }: {
      systemd.services.sdbisect.serviceConfig = {
        Type = "oneshot";
        ExecStart = "${pkgs.coreutils}/bin/true";
        DynamicUser = true;
        PrivatePIDs = true;
        StateDirectory = "sdbisect";
        RemainAfterExit = true;
      };

      networking.useDHCP = false;
      networking.interfaces.eth0.useDHCP = false;
      system.stateVersion = "24.11";
    };

    reverted = { pkgs, ... }: {
      systemd.package = revertedSystemd;
      systemd.services.sdbisect.serviceConfig = {
        Type = "oneshot";
        ExecStart = "${pkgs.coreutils}/bin/true";
        DynamicUser = true;
        PrivatePIDs = true;
        StateDirectory = "sdbisect";
        RemainAfterExit = true;
      };

      networking.useDHCP = false;
      networking.interfaces.eth0.useDHCP = false;
      system.stateVersion = "24.11";
    };
  };

  testScript = ''
    start_all()

    stock.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
    stock.fail("systemctl start sdbisect.service")
    stock.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'Failed to allocate user namespace'")
    stock.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'status=226/NAMESPACE'")

    reverted.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
    reverted.succeed("readlink -f /proc/1/exe | grep -F '${revertedSystemd}'")
    reverted.fail("systemctl start sdbisect.service")
    reverted.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'Failed to allocate user namespace'")
    reverted.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'status=226/NAMESPACE'")
  '';
}
