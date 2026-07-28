{ pkgs, cix }:

let
  nginx = import ../examples/nginx { inherit pkgs; };
  postgres = import ../examples/postgres { inherit pkgs; };
in
pkgs.testers.runNixOSTest {
  name = "vm-dogfood";

  nodes.machine = { ... }: {
    environment.systemPackages = [ cix pkgs.curl ];

    networking.useDHCP = false;
    networking.interfaces.eth0.useDHCP = false;
    networking.firewall.enable = false;
    system.stateVersion = "24.11";
  };

  testScript = ''
    start_all()

    nginx_unit = machine.succeed("cix run ${nginx} --detach").strip()
    machine.wait_until_succeeds("curl --fail --silent http://127.0.0.1:8080/ | grep -F 'hello from composix'")
    machine.succeed("cix ps | grep -F " + nginx_unit)
    machine.succeed("systemctl stop " + nginx_unit)

    postgres_unit = machine.succeed("cix run ${postgres} --detach").strip()
    machine.wait_until_succeeds(
        "${postgres}/bin/psql --host=127.0.0.1 --port=5432 --username=cix --dbname=postgres --no-password --tuples-only --no-align --command='SELECT 1' | grep -Fx 1"
    )
    machine.succeed("cix ps | grep -F " + postgres_unit)
    machine.succeed("systemctl stop " + postgres_unit)

    machine.succeed("systemctl stop cix-run.slice")
    machine.succeed("test -z \"$(systemctl list-units --no-legend 'cix-*' | awk 'NF { print $1 }')\"")
  '';
}
