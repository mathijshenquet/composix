use cix_cixfile::*;

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
        ..
    } = &parsed.builders["build"].steps[0]
    else {
        panic!("expected continued RUN");
    };
    assert_eq!(*line, 9);
    assert!(source.starts_with("RUN printf"));
    assert!(
        matches!(command, NodeCommand::Legacy(command) if command.literal_value().as_deref() == Some("printf '%s\\n' '# inline shell comment text is data' > continued")),
    );
    let BuildStep::Run { command, line, .. } = &parsed.builders["build"].steps[1] else {
        panic!("expected heredoc RUN");
    };
    assert_eq!(*line, 11);
    assert!(matches!(
        command,
        NodeCommand::Legacy(Template { parts }) if matches!(parts.as_slice(),
        [
            TemplatePart::Literal(first),
            TemplatePart::Package { line: 13, .. },
            TemplatePart::Literal(last),
        ] if first.starts_with("# This comment belongs") && last.ends_with(" > result\n"))
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
fn builder_env_is_ordered_plain_text_and_exec_argv_is_quote_aware() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nBUILDER build\nIMPORT ${pkgs.bash}\nENV COREPACK_HOME=$PWD/.corepack\nRUN printf '%s\\n' ok\nSERVICE web\nSTART ${pkgs.nginx}/bin/nginx -g 'daemon off;'\n",
    )
    .unwrap();
    assert!(matches!(
        &parsed.builders["build"].steps[0],
        BuildStep::Env { name, value, .. } if name == "COREPACK_HOME" && value.literal_value().as_deref() == Some("$PWD/.corepack")
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
