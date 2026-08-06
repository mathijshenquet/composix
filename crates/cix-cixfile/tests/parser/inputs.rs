use cix_cixfile::*;

#[test]
fn epoch_nodes_and_phase_braces_parse_with_precise_errors() {
    let input = "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nLET VERSION = '1.2.3'\nARG FLAVOR from plain debug\nBUILDER build {\nIMPORT ${pkgs.bash}\nRUN ${pkgs.coreutils}/bin/printf ${VERSION}\n  WITH MODE=${FLAVOR}\nRUN ${pkgs.bash}/bin/bash <<EOF\nprintf '%s' \"$MODE\" > result\nEOF\n  WITH UNSAFE IGNORE cache\n}\nITEM result {\nCOPY ${build}/result /result\n}\n";
    let parsed = parse(input).unwrap();
    assert_eq!(parsed.lets["VERSION"], ["1.2.3"]);
    assert_eq!(parsed.args["FLAVOR"].selected, "plain");
    assert!(matches!(
        parsed.builders["build"].steps[0],
        BuildStep::Run {
            command: NodeCommand::Argv(_),
            ..
        }
    ));
    assert!(matches!(
        parsed.builders["build"].steps[1],
        BuildStep::Run {
            command: NodeCommand::Heredoc { .. },
            ..
        }
    ));
    let error =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build {\nRUN true\n")
            .unwrap_err();
    assert_eq!(
        error.message,
        "BUILDER build opened at line 2 is never closed"
    );
    let error = parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build {\nRUN echo $X\n}\nITEM result {\nCOPY ${build}/x /x\n}\n").unwrap_err();
    assert_eq!(error.message, "$X is shell interpolation, not argv syntax; declare LET X = value and write ${X}, or use a heredoc");
}

#[test]
fn fetch_expect_is_trailing_and_rejects_the_removed_leading_form() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient ${pkgs.coreutils}/bin/printf top EXPECT sha256-top\nBUILDER build\nIMPORT ${pkgs.bash}\nFETCH printf step EXPECT sha256-step\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap();
    assert_eq!(
        parsed.fetches["ingredient"].expected.as_deref(),
        Some("sha256-top")
    );
    let BuildStep::Fetch { expected, .. } = &parsed.builders["build"].steps[0] else {
        panic!("expected in-builder FETCH");
    };
    assert_eq!(expected.as_deref(), Some("sha256-step"));

    let error = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient ${pkgs.coreutils}/bin/printf payload EXPECT not-sri\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert_eq!(error.line, 2);
    assert!(error.message.contains("SRI sha256"), "{error}");

    let old = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT sha256-old printf payload\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(
        old.message.contains("leading FETCH EXPECT was removed"),
        "{old}"
    );
}

#[test]
fn from_overlays_are_ordered_package_universe_inputs() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable OVERLAY ./first.nix OVERLAY ./second.nix AS pkgs\nFROM github:NixOS/nixpkgs/nixos-25.05 AS stable\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap();
    assert_eq!(
        parsed.inputs["pkgs"].overlays,
        ["./first.nix", "./second.nix"]
    );
    assert!(parsed.inputs["stable"].overlays.is_empty());

    let error = parse(
        "FROM github:owner/source OVERLAY ./overlay.nix AS src\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(error.message.contains("package universe"), "{error}");
}

#[test]
fn names_share_one_namespace_and_references_are_backward_only() {
    let duplicate =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER pkgs\nRUN true\nSERVICE app\nSTART /bin/true\n")
            .unwrap_err();
    assert_eq!(duplicate.line, 2);
    assert!(duplicate.message.contains("line 1"), "{duplicate}");
    assert!(
        duplicate.message.contains("share one namespace"),
        "{duplicate}"
    );

    let forward = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER final\nCOPY ${prior}/x x\nBUILDER prior\nCOPY x x\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert_eq!(forward.line, 3);
    assert!(forward.message.contains("backward-only"), "{forward}");

    let cycle = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY ${build}/x x\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert_eq!(cycle.line, 3);
    assert!(cycle.message.contains("cannot reference itself"), "{cycle}");
}

#[test]
fn source_and_package_interpolation_are_distinct() {
    let tree_attr = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM github:owner/repo AS src\nSERVICE app\nCOPY ${src.subdir} subdir\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(tree_attr.message.contains("${src}/<path>"), "{tree_attr}");

    let universe_tree =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY ${pkgs} pkgs\nSTART /bin/true\n")
            .unwrap_err();
    assert!(
        universe_tree.message.contains("needs an attribute path"),
        "{universe_tree}"
    );
}

#[test]
fn from_local_is_optional_but_a_package_universe_is_required() {
    parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY payload /payload\nSTART /bin/true\n")
        .unwrap();
    let error =
        parse("FROM . AS src\nSERVICE data\nCOPY ${src}/payload /payload\nSTART /bin/true\n")
            .unwrap_err();
    assert!(error.message.contains("package universe"), "{error}");
}

#[test]
fn from_cix_item_is_an_artifact_tree_with_d65_errors() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nSERVICE app\nCOPY ${webvault}/payload /payload\nSTART /bin/true\n",
    )
    .unwrap();
    assert_eq!(parsed.inputs["webvault"].kind, InputKind::Artifact);
    assert_eq!(parsed.inputs["webvault"].url, "family/web:v3");

    let untagged = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web AS webvault\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(untagged.message.contains("flakeref"), "{untagged}");
    assert!(untagged.message.contains(":latest"), "{untagged}");

    let attr_use = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nSERVICE app\nCOPY ${webvault.payload} /payload\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(
        attr_use.message.contains("docs/cixfile.md#inputs"),
        "{attr_use}"
    );

    let import = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM family/web:v3 AS webvault\nBUILDER build\nIMPORT ${webvault}\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(
        import.message.contains("docs/cixfile.md#inputs"),
        "{import}"
    );
}
#[test]
fn from_lock_metadata_is_a_builder_env_template() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM github:owner/repository/rev AS src\nBUILDER build\nIMPORT ${pkgs.bash}\nENV GIT_COMMIT_HASH=${src.rev}\nRUN true\nITEM app\nCOPY ${build}/out /out\n",
    )
    .unwrap();
    assert!(matches!(
        &parsed.builders["build"].steps[0],
        BuildStep::Env { value, .. }
            if matches!(value.parts.as_slice(), [TemplatePart::InputMetadata { namespace, attribute, .. }] if namespace == "src" && attribute == "rev")
    ));
}
