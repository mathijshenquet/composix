{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  deviceProbe = pkgs.runCommand "scenario-device-probe" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/device-probe" <<'SH'
    #!${pkgs.runtimeShell}
    set -eu
    ${pkgs.coreutils}/bin/head -c 1 /dev/cix-device > /dev/null
    ! ${pkgs.coreutils}/bin/head -c 1 /dev/zero > /dev/null 2>&1
    ${pkgs.util-linux}/bin/findmnt -n -o OPTIONS --target /dev/shm | ${pkgs.gnugrep}/bin/grep -E '(^|,)size=(64M|65536k)(,|$)'
    echo device-probe-ok
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/device-probe"
    cat > "$out/cix-manifest.json" <<'EOF'
    {"cixManifest":0,"start":["bin/device-probe"],"claims":["gpu",{"device":"/dev/cix-device"}],"shm":"64M"}
    EOF
  '';
in
scenario.node ''
  machine.succeed("mknod -m 666 /dev/cix-device c 1 3")
  item = machine.succeed("nix-store --add ${deviceProbe}").strip()
  unit = machine.succeed("cix run " + item + " --detach").strip()
  machine.succeed("systemctl is-active " + unit)
  machine.succeed("sleep 2; systemctl is-active " + unit)
  machine.succeed("systemctl show " + unit + " --property=DevicePolicy --value | grep -Fx closed")
  machine.succeed("systemctl show " + unit + " --property=DeviceAllow --value | grep -F /dev/dri")
  machine.succeed("systemctl show " + unit + " --property=DeviceAllow --value | grep -F /dev/cix-device")
  machine.succeed("systemctl show " + unit + " --property=SupplementaryGroups --value | grep -E '(^| )video( |$)'")
  machine.succeed("systemctl show " + unit + " --property=SupplementaryGroups --value | grep -E '(^| )render( |$)'")
  machine.succeed("systemctl show " + unit + " --property=PrivateDevices --value | grep -Fx no")
  machine.succeed("systemctl stop " + unit)
''
