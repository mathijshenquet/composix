use cix_cixfile::*;

#[test]
fn import_accepts_whole_package_refs_and_path_has_migration_errors() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nIMPORT ${pkgs.coreutils}\nRUN true\nSERVICE service\nIMPORT ${pkgs.bash}\nIMPORT ${build}\nSTART bash\nAPP app\nIMPORT ${pkgs.coreutils}\nSTART true\nITEM item\nIMPORT ${pkgs.bash}\n",
    )
    .unwrap();
    assert_eq!(parsed.builders["build"].imports.len(), 2);
    assert_eq!(parsed.artifacts["service"].imports.len(), 2);
    assert_eq!(parsed.artifacts["app"].imports.len(), 1);
    assert_eq!(parsed.artifacts["item"].imports.len(), 1);

    let suffix = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}/bin\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(
        suffix.message.contains("whole package references"),
        "{suffix}"
    );

    let builder_path = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nPATH ${pkgs.bash}/bin\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(
        builder_path.message.contains("use IMPORT"),
        "{builder_path}"
    );
    let service_path =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nPATH ${pkgs.bash}/bin\nSTART /bin/true\n")
            .unwrap_err();
    assert_eq!(
        service_path.message,
        "PATH was removed; use ENV PATH=<value>; see docs/cixfile.md#runtime-path"
    );
}

#[test]
fn bare_and_explicit_local_copy_sources_coexist() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nBUILDER build\nCOPY bare.txt bare\nCOPY ${src}/explicit.txt explicit\nSERVICE app\nCOPY bare.txt /bare\nCOPY ${src}/explicit.txt /explicit\nSTART /bin/true\n",
    )
    .unwrap();
    let BuildStep::Copy(bare) = &parsed.builders["build"].steps[0] else {
        panic!("expected COPY");
    };
    assert_eq!(bare.src, Template::literal("bare.txt"));
    assert!(matches!(
        parsed.builders["build"].steps[1],
        BuildStep::Copy(Copy { .. })
    ));
    assert_eq!(parsed.artifacts["app"].copies.len(), 2);
}

#[test]
fn store_copy_mode_and_structural_materialization_are_static() {
    let parsed = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
COPY seed seed
SERVICE linked
COPY ${pkgs.coreutils} /toolset
START ${pkgs.coreutils}/bin/true
SERVICE tomcat
COPY ${build}/tomcat /tomcat
COPY patch /tomcat/conf/patch
START ${pkgs.coreutils}/bin/true
SERVICE directus
COPY ${build}/dist /app
STATEDIR /app/database
DIR /app/uploads
START ${pkgs.coreutils}/bin/true
SERVICE file-write
COPY ${pkgs.coreutils} /app
FILE /app/config <<EOF
configured
EOF
START ${pkgs.coreutils}/bin/true
SERVICE local
COPY ${src}/tree /tree
START ${pkgs.coreutils}/bin/true
"#,
    )
    .unwrap();

    assert_eq!(parsed.artifacts["linked"].copies[0].mode, CopyMode::Link);
    assert_eq!(
        parsed.artifacts["tomcat"].copies[0].mode,
        CopyMode::Materialize
    );
    assert_eq!(
        parsed.artifacts["directus"].copies[0].mode,
        CopyMode::Materialize
    );
    assert_eq!(
        parsed.artifacts["file-write"].copies[0].mode,
        CopyMode::Materialize
    );
    assert_eq!(
        parsed.artifacts["local"].copies[0].mode,
        CopyMode::Materialize
    );
}

#[test]
fn bare_artifact_commands_need_no_explicit_path() {
    let parsed = parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART_PRE setup\nSTART app\n").unwrap();
    assert_eq!(
        parsed.artifacts["app"].service.start[0].literal_value(),
        Some("app".into())
    );
    assert_eq!(
        parsed.artifacts["app"].service.start_pre.as_ref().unwrap()[0].literal_value(),
        Some("setup".into())
    );
}

#[test]
fn script_has_the_d55_migration_error_and_is_not_an_alias() {
    let source = "SCRIPT bin/start <<EOF";
    let error = parse(&format!(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\n{source}\ntrue\nEOF\n"
    ))
    .unwrap_err();
    assert_eq!(error.line, 4);
    assert_eq!(error.source, source);
    assert_eq!(
        error.message,
        "SCRIPT was removed; COPY a script and invoke it with START ${pkgs.bash}/bin/sh /path; see docs/cixfile.md#copy"
    );
}

#[test]
fn cachedir_parses_and_link_teaches_its_removal() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY ${pkgs.hello}/bin/hello /bin/hello\nSTART /bin/hello\nCACHEDIR /var/cache/app\n",
    )
    .unwrap();
    assert!(parsed.artifacts["app"]
        .service
        .dirs
        .cache
        .contains("/var/cache/app"));
    let copy = &parsed.artifacts["app"].copies[0];
    assert_eq!(copy.dst, "bin/hello");
    assert_eq!(copy.mode, CopyMode::Link);
    assert!(matches!(
        copy.src.parts.first(),
        Some(TemplatePart::Package { attrpath, .. }) if attrpath == "hello"
    ));

    let cache =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\nCACHE /var/cache/app\n")
            .unwrap_err();
    assert_eq!(cache.line, 4);
    assert_eq!(
        cache.message,
        "CACHE was removed; delete this line because builder workspaces persist automatically; see docs/cixfile.md#builders"
    );

    let link = parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nLINK ${pkgs.hello}/bin/hello /bin/hello\nSTART /bin/hello\n").unwrap_err();
    assert_eq!(link.line, 3);
    assert!(link.message.contains("LINK was removed"));
    assert!(link
        .message
        .contains("COPY <source> <absolute-destination>"));
}

#[test]
fn builder_destinations_are_relative_and_artifact_destinations_are_absolute() {
    parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY . .\nSERVICE app\nCOPY ${build} /\nSTART /bin/true\n",
    )
    .unwrap();
    for (destination, spelling) in [("relative", "/relative"), (".", "/")] {
        let input = format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY payload {destination}\nSTART /bin/true\n"
        );
        let error = parse(&input).unwrap_err();
        assert_eq!(error.line, 3);
        assert!(
            error.message.contains("absolute inside the item"),
            "{destination}: {error}"
        );
        assert!(error.message.contains(spelling), "{destination}: {error}");
    }
    let builder = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY payload /bad\nSERVICE app\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(builder.message.contains("workdir-relative"), "{builder}");
    assert!(builder.message.contains("write bad"), "{builder}");

    for directive in [
        "FILE etc/app.conf <<EOF\nvalue\nEOF",
        "COPY /nix/store/target bin/tool",
        "START bin/tool",
        "START_PRE bin/tool\nSTART /bin/true",
    ] {
        let input = format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n{directive}\nSTART /bin/true\n");
        let error = parse(&input).unwrap_err();
        assert!(
            error.message.contains("must be absolute"),
            "{directive}: {error}"
        );
        assert!(error.message.contains("/bin") || error.message.contains("/etc"));
    }
}

#[test]
fn builder_copies_are_sequential_but_artifact_destinations_stay_unique() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nCOPY first value\nRUN true\nCOPY second value\nITEM result\nCOPY ${build}/value /value\n",
    )
    .unwrap();
    assert_eq!(parsed.builders["build"].steps.len(), 3);

    let duplicate = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM result\nCOPY first /value\nCOPY second /value\n",
    )
    .unwrap_err();
    assert_eq!(duplicate.line, 4);
    assert!(
        duplicate.message.contains("already populated"),
        "{duplicate}"
    );
}
