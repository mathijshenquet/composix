//! Parser acceptance coverage lives outside the implementation modules.

#[cfg(test)]
mod d47_tests {
    use cix_cixfile::*;
    use std::collections::BTreeSet;

    #[test]
    fn parses_blocks_binders_and_both_artifact_kinds() {
        let parsed = parse(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
FETCH ingredient ${pkgs.coreutils}/bin/printf payload
BUILDER build
IMPORT ${pkgs.bash}
COPY Cargo.toml Cargo.toml
FETCH printf fetched > fetched
RUN cp fetched built
SERVICE web
COPY ${build}/built /bin/web
FILE /etc/app.conf <<E
source=${src}
E
	LINK ${pkgs.bash}/bin/bash /bin/sh
	ENV PATH = bin
START web
START_PRE /bin/web
ENV PORT = 8080 required
PORT http = $PORT
LISTENER admin
STATEDIR /var/lib/web
	CACHEDIR /var/cache/web
LOGDIR /var/log/web
CONFIGDIR /etc/web
RUNDIR /run/web
CLAIM jit
CLAIM egress
APP migrate
COPY ${ingredient} /payload
START /bin/true
ENV MODE = once
STATEDIR /var/lib/migrate
	CACHEDIR /var/cache/migrate
	CLAIM egress
	"#,
        )
        .unwrap();
        assert_eq!(parsed.fetch_order, ["ingredient"]);
        assert_eq!(parsed.builder_order, ["build"]);
        assert_eq!(parsed.artifact_order, ["web", "migrate"]);
        assert_eq!(parsed.builders["build"].steps.len(), 3);
        assert_eq!(parsed.artifacts["web"].kind, ArtifactKind::Service);
        assert_eq!(parsed.artifacts["migrate"].kind, ArtifactKind::App);
    }

    #[test]
    fn fetch_expect_parses_in_both_forms_and_validates_the_hash() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT sha256-top ${pkgs.coreutils}/bin/printf top\nBUILDER build\nIMPORT ${pkgs.bash}\nFETCH EXPECT sha256-step printf step\nSERVICE app\nSTART /bin/true\n",
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
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT not-sri printf payload\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.message.contains("SRI sha256"), "{error}");
    }

    #[test]
    fn import_accepts_whole_package_refs_and_path_has_migration_errors() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nIMPORT ${pkgs.coreutils}\nRUN true\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap();
        assert_eq!(parsed.builders["build"].imports.len(), 2);

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
            "PATH was removed; use ENV PATH = <value>; see docs/cixfile.md#runtime-path"
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
    fn migration_errors_name_the_d47_rewrite() {
        for (input, line, message) in [
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nRUN true\nSERVICE app\nSTART /bin/true\n",
                2,
                "RUN is outside a BUILDER",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nCOPY ${build}/bin/app /bin/app\nSTART /bin/app\n",
                3,
                "no binder named `build`; name your builder: `BUILDER build`",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nTAKE bin/app /bin/app\nSTART /bin/app\n",
                3,
                "TAKE was removed",
            ),
            (
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nPATH ${pkgs.bash}/bin\nSERVICE app\nSTART bash\n",
                2,
                "PATH was removed; use IMPORT",
            ),
        ] {
            let error = parse(input).unwrap_err();
            assert_eq!(error.line, line, "{error}");
            assert!(error.message.contains(message), "{error}");
            assert!(error.to_string().contains(&format!("{:?}", error.source)));
        }
    }

    #[test]
    fn outbound_has_a_d48_migration_error_and_is_not_an_alias() {
        let error =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\nOUTBOUND\n").unwrap_err();
        assert_eq!(error.line, 4);
        assert!(error.message.contains("CLAIM egress"), "{error}");
        assert!(error.message.contains("docs/cixfile.md#claims"), "{error}");
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
    fn comments_continuations_and_run_heredocs_preserve_shell_text() {
        let parsed = parse(
            r#"# The package universe is intentionally split across physical lines.
FROM github:NixOS/nixpkgs/nixos-unstable \
    AS pkgs

BUILDER build
IMPORT ${pkgs.bash} \
    ${pkgs.coreutils}
# This comment is ignored by the Cixfile parser.
RUN printf '%s\n' \
    '# inline shell comment text is data' > continued
RUN <<SCRIPT
# This comment belongs to the builder shell.
printf '%s\n' ${pkgs.hello} > result
SCRIPT

SERVICE app
START /bin/true \
    # this is an argument, not a Cixfile comment
"#,
        )
        .unwrap();
        assert_eq!(parsed.builders["build"].imports.len(), 2);
        let BuildStep::Run {
            command,
            line,
            source,
        } = &parsed.builders["build"].steps[0]
        else {
            panic!("expected continued RUN");
        };
        assert_eq!(*line, 9);
        assert!(source.starts_with("RUN printf"));
        assert_eq!(
            command.literal_value().as_deref(),
            Some("printf '%s\\n' '# inline shell comment text is data' > continued")
        );
        let BuildStep::Run { command, line, .. } = &parsed.builders["build"].steps[1] else {
            panic!("expected heredoc RUN");
        };
        assert_eq!(*line, 11);
        assert!(matches!(
            command.parts.as_slice(),
            [
                TemplatePart::Literal(first),
                TemplatePart::Package { line: 13, .. },
                TemplatePart::Literal(last),
            ] if first.starts_with("# This comment belongs") && last.ends_with(" > result\n")
        ));
        let start = &parsed.artifacts["app"].service.start;
        assert_eq!(start[1].literal_value().as_deref(), Some("#"));
        assert_eq!(
            start.last().and_then(Template::literal_value).as_deref(),
            Some("comment")
        );
    }

    #[test]
    fn run_heredoc_errors_use_physical_body_lines() {
        let error = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nRUN <<SCRIPT\ntrue\nprintf ${missing}\nSCRIPT\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(error.line, 5, "{error}");
        assert_eq!(error.source, "printf ${missing}");

        let dangling = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true \\\n",
        )
        .unwrap_err();
        assert_eq!(dangling.line, 3, "{dangling}");
        assert!(dangling.message.contains("continuation"), "{dangling}");

        let continued = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash} \\\n    ${missing.tool}\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap_err();
        assert_eq!(continued.line, 4, "{continued}");
        assert_eq!(continued.source.trim(), "${missing.tool}");
    }

    #[test]
    fn cixfile_comments_are_full_line_only() {
        let parsed = parse(
            "  # ignored before the first declaration \\\nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n# ignored in a block\nSTART /bin/echo #kept\n",
        )
        .unwrap();
        let start = &parsed.artifacts["app"].service.start;
        assert_eq!(start.len(), 2);
        assert_eq!(start[1].literal_value().as_deref(), Some("#kept"));
    }

    #[test]
    fn cachedir_and_link_use_the_d52_spellings() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nLINK ${pkgs.hello}/bin/hello /bin/hello\nSTART /bin/hello\nCACHEDIR /var/cache/app\n",
        )
        .unwrap();
        assert!(parsed.artifacts["app"]
            .service
            .dirs
            .cache
            .contains("/var/cache/app"));
        let Assembly::Link { dst, target } = &parsed.artifacts["app"].assembly[0] else {
            panic!("expected LINK");
        };
        assert_eq!(dst, "bin/hello");
        assert!(matches!(
            target.parts.first(),
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

        let link = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nLINK bin/hello ${pkgs.hello}/bin/hello\nSTART /bin/hello\n",
        )
        .unwrap_err();
        assert_eq!(link.line, 3);
        assert!(link.message.contains("arguments are target then link path"));
        assert!(link.message.contains("LINK <target> <absolute-linkpath>"));
    }

    #[test]
    fn builder_env_is_ordered_plain_text_and_exec_argv_is_quote_aware() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nENV COREPACK_HOME = $PWD/.corepack\nRUN printf '%s\\n' ok\nSERVICE web\nSTART ${pkgs.nginx}/bin/nginx -g 'daemon off;'\n",
        )
        .unwrap();
        assert!(matches!(
            &parsed.builders["build"].steps[0],
            BuildStep::Env { name, value, .. } if name == "COREPACK_HOME" && value == "$PWD/.corepack"
        ));
        assert_eq!(
            parsed.artifacts["web"].service.start[2]
                .literal_value()
                .as_deref(),
            Some("daemon off;")
        );

        let unterminated = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART ${pkgs.nginx}/bin/nginx -g 'daemon off;\n",
        )
        .unwrap_err();
        assert_eq!(unterminated.line, 3);
        assert!(
            unterminated.message.contains("unterminated quote"),
            "{unterminated}"
        );
    }

    #[test]
    fn role_directory_directives_and_claim_are_hard_migrations() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /var/lib/web\nCACHEDIR /var/cache/web\nLOGDIR /var/log/web\nCONFIGDIR /etc/web\nRUNDIR /run/web\nCLAIM jit\nCLAIM egress\n",
        )
        .unwrap();
        let dirs = &parsed.artifacts["web"].service.dirs;
        assert!(dirs.state.contains("/var/lib/web"));
        assert!(dirs.cache.contains("/var/cache/web"));
        assert!(dirs.logs.contains("/var/log/web"));
        assert!(dirs.config.contains("/etc/web"));
        assert!(dirs.run.contains("/run/web"));
        assert_eq!(
            parsed.artifacts["web"].service.claims,
            BTreeSet::from([Claim::Named("egress".into()), Claim::Named("jit".into()),])
        );
        for (directive, replacement, anchor) in [
            ("STATE /var/lib/web", "STATEDIR", "#role-dirs"),
            ("LOGS /var/log/web", "LOGDIR", "#role-dirs"),
            ("CONFIG /etc/web", "CONFIGDIR", "#role-dirs"),
            ("JIT", "CLAIM jit", "#claims"),
            ("EGRESS", "CLAIM egress", "#claims"),
        ] {
            let error = parse(&format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{directive}\n"
            ))
            .unwrap_err();
            assert_eq!(error.line, 4);
            assert!(error.message.contains(replacement), "{error}");
            assert!(error.message.contains(anchor), "{error}");
        }
        let unknown =
            parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nCLAIM all\n").unwrap_err();
        assert!(unknown.message.contains("jit, egress"), "{unknown}");
    }

    #[test]
    fn device_gpu_and_shm_claims_are_strict() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE detector\nSTART /bin/true\nCLAIM gpu\nCLAIM device /dev/video0\nSHM 256M\n",
        )
        .unwrap();
        let service = &parsed.artifacts["detector"].service;
        assert!(service.claims.contains(&Claim::Named("gpu".into())));
        assert!(service
            .claims
            .contains(&Claim::Device("/dev/video0".into())));
        assert_eq!(service.shm.as_deref(), Some("256M"));
        let manifest: serde_json::Value =
            serde_json::from_str(&generate_spec_json(&parsed).unwrap()).unwrap();
        assert_eq!(
            manifest["claims"],
            serde_json::json!(["gpu", {"device": "/dev/video0"}])
        );
        assert_eq!(manifest["shm"], "256M");

        for directive in [
            "CLAIM device ttyUSB0",
            "CLAIM device /dev/../etc/passwd",
            "CLAIM device /tmp/device",
            "CLAIM gpu extra",
            "SHM -1G",
            "SHM 1Z",
            "SHM 1G extra",
        ] {
            let error = parse(&format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE detector\nSTART /bin/true\n{directive}\n"
            ))
            .unwrap_err();
            assert_eq!(error.line, 4, "{directive}: {error}");
        }
    }

    #[test]
    fn directories_accept_arbitrary_paths_and_dir_modes() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /srv/web/state\nCACHEDIR /app/cache\nLOGDIR /app/logs\nRUNDIR /tmp/web/run\nDIR /media:ro\nDIR /consume:rw\nDIR /scratch\n",
        )
        .unwrap();
        let dirs = &parsed.artifacts["web"].service.dirs;
        assert!(dirs.state.contains("/srv/web/state"));
        assert!(dirs.cache.contains("/app/cache"));
        assert!(dirs.logs.contains("/app/logs"));
        assert!(dirs.run.contains("/tmp/web/run"));
        assert_eq!(dirs.data.get("/media"), Some(&true));
        assert_eq!(dirs.data.get("/consume"), Some(&false));
        assert_eq!(dirs.data.get("/scratch"), Some(&false));

        for declaration in [
            "STATEDIR relative",
            "DIR /data/../escape",
            "DIR /data:other",
        ] {
            let error = parse(&format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{declaration}\n"
            ))
            .unwrap_err();
            assert_eq!(error.line, 4, "{declaration}: {error}");
        }

        let duplicate = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /shared\nDIR /shared:ro\n",
        )
        .unwrap_err();
        assert_eq!(duplicate.line, 5);
        assert!(duplicate.message.contains("duplicated"), "{duplicate}");
    }

    #[test]
    fn app_rejects_service_only_surface_at_the_directive_line() {
        for (directive, message) in [
            ("PORT http = 8080", "PORT is not allowed inside APP"),
            ("LISTENER http", "LISTENER is not allowed inside APP"),
            ("JIT", "replace it with CLAIM jit"),
            ("START_PRE /bin/true", "START_PRE is not allowed inside APP"),
            ("LOGDIR /var/log/job", "LOGDIR is not allowed inside APP"),
            ("CONFIGDIR /etc/job", "CONFIGDIR is not allowed inside APP"),
            ("RUNDIR /run/job", "RUNDIR is not allowed inside APP"),
            ("PATH bin", "PATH was removed; use ENV PATH = <value>"),
        ] {
            let input = format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nAPP job\nSTART /bin/true\n{directive}\n");
            let error = parse(&input).unwrap_err();
            assert_eq!(error.line, 4, "{directive}: {error}");
            assert!(error.message.contains(message), "{directive}: {error}");
        }
    }

    #[test]
    fn item_is_pure_assembly_and_runtime_directives_name_the_d68_seam() {
        let parsed = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\nCOPY payload /payload\nFILE /share/message <<EOF\nhello\nEOF\nLINK ${pkgs.hello}/bin/hello /bin/hello\n",
        )
        .unwrap();
        assert_eq!(parsed.artifacts["data"].kind, ArtifactKind::Item);
        assert_eq!(parsed.artifacts["data"].copies.len(), 1);
        assert_eq!(parsed.artifacts["data"].assembly.len(), 2);

        for directive in [
            "START /bin/hello",
            "START_PRE /bin/hello",
            "ENV PATH = bin",
            "PORT http = 8080",
            "LISTENER http",
            "STATEDIR /var/lib/data",
            "CACHEDIR /var/cache/data",
            "LOGDIR /var/log/data",
            "CONFIGDIR /etc/data",
            "RUNDIR /run/data",
            "CLAIM egress",
            "HEALTH /bin/hello",
        ] {
            let input = format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\n{directive}\n"
            );
            let error = parse(&input).unwrap_err();
            assert_eq!(error.line, 3, "{directive}: {error}");
            assert!(
                error.message.contains("ITEM is content-only"),
                "{directive}: {error}"
            );
            assert!(
                error.message.contains("use SERVICE or APP"),
                "{directive}: {error}"
            );
        }
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
            "LINK /nix/store/target bin/tool",
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
}
