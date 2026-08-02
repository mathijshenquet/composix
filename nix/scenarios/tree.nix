{ pkgs, cix }:

let
  scenario = import ./lib.nix { inherit pkgs cix; };
  treeItem = version: pkgs.runCommand "scenario-tree-item-${version}" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/tree-service" <<'SH'
    #!${pkgs.runtimeShell}
    echo "$TREE_VERSION" > /var/lib/tree/version
    echo "$INSTANCE" > /var/lib/tree/instance
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/tree-service"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["bin/tree-service"],"env":{"TREE_VERSION":{"default":"${version}"},"INSTANCE":{"required":true}},"dirs":{"state":["/var/lib/tree"]}}
    EOF
  '';
  leaf = pkgs.runCommand "scenario-tree-leaf" { } ''
    mkdir -p "$out/bin"
    cat > "$out/bin/tree-leaf" <<'SH'
    #!${pkgs.runtimeShell}
    exec ${pkgs.coreutils}/bin/sleep infinity
    SH
    chmod 0755 "$out/bin/tree-leaf"
    cat > "$out/cix-manifest.json" <<EOF
    {"cixManifest":0,"start":["bin/tree-leaf"]}
    EOF
  '';
  subtree = pkgs.writeTextDir "cix.json" ''
    {
      "cixCompose": 1,
      "name": "artifact-name-is-not-instance-identity",
      "children": {
        "leaf": { "item": "scenario-tree-leaf:v1" }
      }
    }
  '';
  root = pkgs.writeText "scenario-tree-root.json" ''
    {
      "cixCompose": 1,
      "name": "tree",
      "children": {
        "inline": {
          "children": {
            "one": { "item": "scenario-tree-shared:track", "env": { "INSTANCE": "one" } },
            "two": { "item": "scenario-tree-shared:track", "env": { "INSTANCE": "two" } }
          }
        },
        "sealed": { "compose": "scenario-tree-suite:v1" }
      }
    }
  '';
in
scenario.node ''
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${treeItem "v1"}) scenario-tree-shared:track")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${leaf}) scenario-tree-leaf:v1")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${subtree}) scenario-tree-suite:v1")
  machine.succeed("cp ${root} /tmp/scenario/cix.json")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json")
  machine.succeed("systemctl is-active cix-tree-inline-one.service cix-tree-inline-two.service cix-tree-sealed-leaf.service")
  machine.succeed("test $(systemctl show cix-tree-inline-one.service -p Slice --value) = cix-tree-inline.slice")
  machine.succeed("test $(systemctl show cix-tree-sealed-leaf.service -p Slice --value) = cix-tree-sealed.slice")
  machine.succeed("test $(systemctl show cix-tree-inline.slice -p ControlGroup --value) = /cix.slice/cix-tree.slice/cix-tree-inline.slice")
  machine.succeed("test -f /var/lib/cix-tree-inline-one/var/lib/tree/version")
  machine.succeed("test -f /var/lib/cix-tree-inline-two/var/lib/tree/version")
  machine.succeed("test $(cat /var/lib/cix-tree-inline-one/var/lib/tree/instance) = one")
  machine.succeed("test $(cat /var/lib/cix-tree-inline-two/var/lib/tree/instance) = two")
  machine.succeed("test $(stat -c %i /var/lib/cix-tree-inline-one/var/lib/tree) != $(stat -c %i /var/lib/cix-tree-inline-two/var/lib/tree)")
  machine.succeed("jq -e '.paths | keys == [\"inline/one\",\"inline/two\",\"sealed\",\"sealed/leaf\"]' /tmp/scenario/cix.lock")

  one_before = machine.succeed("systemctl show cix-tree-inline-one.service -p ActiveEnterTimestampMonotonic --value").strip()
  two_before = machine.succeed("systemctl show cix-tree-inline-two.service -p ActiveEnterTimestampMonotonic --value").strip()
  leaf_before = machine.succeed("systemctl show cix-tree-sealed-leaf.service -p ActiveEnterTimestampMonotonic --value").strip()
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix tag $(nix store add-path ${treeItem "v2"}) scenario-tree-shared:track")
  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix up /tmp/scenario/cix.json --update-lock inline/one")
  machine.succeed("test $(cat /var/lib/cix-tree-inline-one/var/lib/tree/version) = v2")
  machine.succeed("test $(cat /var/lib/cix-tree-inline-two/var/lib/tree/version) = v1")
  machine.succeed("test $(systemctl show cix-tree-inline-one.service -p ActiveEnterTimestampMonotonic --value) != " + one_before)
  machine.succeed("test $(systemctl show cix-tree-inline-two.service -p ActiveEnterTimestampMonotonic --value) = " + two_before)
  machine.succeed("test $(systemctl show cix-tree-sealed-leaf.service -p ActiveEnterTimestampMonotonic --value) = " + leaf_before)
  machine.succeed("test $(jq -r '.paths[\"inline/one\"].storePath' /tmp/scenario/cix.lock) != $(jq -r '.paths[\"inline/two\"].storePath' /tmp/scenario/cix.lock)")

  machine.succeed("cp /tmp/scenario/cix.json /tmp/scenario/root-edit.json")
  machine.succeed("cix root add inline/temp scenario-tree-shared:track --file /tmp/scenario/root-edit.json")
  machine.succeed("jq -e '.children.inline.children.temp.item == \"scenario-tree-shared:track\"' /tmp/scenario/root-edit.json")
  machine.succeed("cix root remove inline/temp --file /tmp/scenario/root-edit.json")
  machine.succeed("jq -e '.children.inline.children.temp == null' /tmp/scenario/root-edit.json")

  machine.succeed("CIX_STATE_DIR=/var/lib/cix-index cix down tree")
  machine.succeed("systemctl reset-failed 'cix-tree*' || true")
  machine.succeed("test -z \"$(systemctl list-units --all --no-legend 'cix-tree*' | awk 'NF { print $1 }')\"")
''
