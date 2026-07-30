{ pkgs, revertedSystemd, systemd257, kernel617Packages }:

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

    kernel617 = { pkgs, ... }: {
      boot.kernelPackages = kernel617Packages;
      hardware.deviceTree.enable = false;
      system.boot.loader.kernelFile = "bzImage";
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

    kernel618 = { pkgs, ... }: {
      boot.kernelPackages = pkgs.linuxPackages_6_18;
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

    def exercise_kernel_axis(machine, expected_kernel):
        actual_kernel = machine.succeed("uname -r").strip()
        assert actual_kernel.startswith(expected_kernel + "."), actual_kernel
        machine.succeed("systemctl --version | head -1 | grep -E '^systemd 261( |$)'")
        status, output = machine.execute("systemctl start sdbisect.service 2>&1")
        print(expected_kernel + " systemctl start exit=" + str(status) + " output=" + output)
        if status == 0:
            machine.succeed("systemctl is-active --quiet sdbisect.service")
            print(expected_kernel + " uid-map triple passed")
            return "passed"

        assert status == 1, expected_kernel + " unexpected systemctl exit: " + str(status)
        machine.wait_until_succeeds("journalctl -u sdbisect.service -b --no-pager | grep -F 'Failed to write UID map: Operation not permitted'")
        journal = machine.succeed("journalctl -u sdbisect.service -b --no-pager -o short-monotonic")
        print(expected_kernel + " uid-map triple failed:\\n" + journal)
        assert "Failed to write UID map: Operation not permitted" in journal
        assert "status=226/NAMESPACE" in journal
        return "uid-map-eperm"

    kernel617_result = exercise_kernel_axis(kernel617, "6.17")
    kernel618_result = exercise_kernel_axis(kernel618, "6.18")
    assert kernel617_result == "uid-map-eperm", "6.17 control no longer reproduces"
    assert kernel618_result == "uid-map-eperm", "6.18 control no longer reproduces"
    print("kernel axis result: 6.17=" + kernel617_result + ", 6.18=" + kernel618_result)
  '';
}
