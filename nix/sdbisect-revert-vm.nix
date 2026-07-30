{ pkgs, revertedSystemd, systemd257 }:

pkgs.testers.runNixOSTest {
  name = "sdbisect-revert";

  nodes = {
    stock = { pkgs, ... }: {
      systemd.settings.Manager.LogLevel = "debug";

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

    v257 = { pkgs, ... }: {
      boot.initrd.systemd.package = pkgs.systemd;
      systemd.package = systemd257;
      systemd.settings.Manager.LogLevel = "debug";
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
    serial_stdout_off()
    start_all()

    stock.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
    stock.succeed("${pkgs.strace}/bin/strace -ff -o /tmp/sdbisect-strace -p 1 -e trace=%process,mount_setattr,openat,write,unshare,setns >/tmp/sdbisect-strace.stderr 2>&1 & echo $! > /tmp/sdbisect-strace.pid")
    stock.succeed("sleep 1")
    stock.fail("systemctl start sdbisect.service")
    stock.succeed("kill -INT $(cat /tmp/sdbisect-strace.pid)")
    stock.succeed("sleep 1")
    stock.succeed("grep -h -A 1 -E '/proc/[0-9]+/uid_map' /tmp/sdbisect-strace.* | grep -F 'EPERM (Operation not permitted)'")
    print(stock.succeed("grep -h -E 'uid_map|EPERM|setgroups|gid_map|mount_setattr' /tmp/sdbisect-strace.*"))
    stock.succeed("journalctl -b --no-pager -o short-monotonic > /tmp/sdbisect-stock-journal.txt")
    print(stock.succeed("journalctl -u sdbisect.service -b --no-pager -o short-monotonic | grep -E 'sdbisect|namespace|map|setgroups|id-mapped|Operation not permitted'"))
    stock.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'Failed to allocate user namespace'")
    stock.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'status=226/NAMESPACE'")

    reverted.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
    reverted.succeed("readlink -f /proc/1/exe | grep -F '${revertedSystemd}'")
    reverted.fail("systemctl start sdbisect.service")
    reverted.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'Failed to allocate user namespace'")
    reverted.succeed("journalctl -u sdbisect.service -b --no-pager | grep -F 'status=226/NAMESPACE'")

    v257.succeed("systemctl --version | head -1 | grep -E '^systemd 257( |$)'")
    v257.succeed("readlink -f /proc/1/exe | grep -F '${systemd257}'")
    v257.fail("systemctl start sdbisect.service")
    print(v257.succeed("journalctl -u sdbisect.service -b --no-pager -o short-monotonic"))
  '';
}
