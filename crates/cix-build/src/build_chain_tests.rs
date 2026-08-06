use super::*;
use crate::evaluation::{NixEvaluation, ResolvedCommand};
use crate::fetch::{
    concrete_fetch_url, revoke_from_store, token_matches, url_prefix, Consent, ConsentStore,
    CredentialToken,
};
use crate::fetch_state::FetchState;
use crate::memo::{
    test_compare_cold_paths as compare_cold_paths, test_copy_key_arguments as copy_key_arguments,
    test_memo_entry as memo_entry, test_retain_fetch_output_roots as retain_fetch_output_roots,
    test_step_key as step_key, test_step_memo_key as step_memo_key,
    test_top_fetch_chain_key as top_fetch_chain_key, test_validate_step_memo as validate_step_memo,
    test_verify_cold_read_set as verify_cold_read_set, StepKeyRequest,
};
use crate::sandbox::{
    failure_message as sandbox_failure, failure_problem_hints, prepare_import_union, RunNetwork,
    Sandbox, SandboxRequest,
};
use crate::workspace::{memo_output_hashes, revert_step_writes, stage_input, workspace_identity};
use crate::{ConsumedPath, Copy, StepChange};
use std::fs;
use std::path::PathBuf;

fn closure(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

#[test]
fn step_key_tracks_chain_inputs_without_workdir_bytes() {
    let environment = BTreeMap::from([("PATH".into(), "/nix/store/tool/bin".into())]);
    let base = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "cargo build",
        offered_closure: &closure(&["/nix/store/tool"]),
        ordered_imports: &[],
        predecessor: "previous-key",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &[],
    })
    .unwrap();
    assert_eq!(
        base,
        step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo build",
            offered_closure: &closure(&["/nix/store/tool"]),
            ordered_imports: &[],
            predecessor: "previous-key",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &[],
        })
        .unwrap()
    );
    assert_ne!(
        base,
        step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo test",
            offered_closure: &closure(&["/nix/store/tool"]),
            ordered_imports: &[],
            predecessor: "previous-key",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &[],
        })
        .unwrap()
    );
    let changed_environment = BTreeMap::from([("PATH".into(), "/nix/store/other-tool/bin".into())]);
    assert_ne!(
        base,
        step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo build",
            offered_closure: &closure(&["/nix/store/tool"]),
            ordered_imports: &[],
            predecessor: "previous-key",
            declared_sources: &[],
            environment: &changed_environment,
            fetch_pin: None,
            universe_identities: &[],
        })
        .unwrap()
    );
    assert_ne!(
        base,
        step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo build",
            offered_closure: &closure(&["/nix/store/new-tool"]),
            ordered_imports: &[],
            predecessor: "previous-key",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &[],
        })
        .unwrap()
    );
    assert_ne!(
        base,
        step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo build",
            offered_closure: &closure(&["/nix/store/tool"]),
            ordered_imports: &[],
            predecessor: "changed-predecessor",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &[],
        })
        .unwrap()
    );
}

#[test]
fn copy_source_hash_and_fetch_pin_participate_in_chain_keys() {
    let environment = BTreeMap::new();
    let offered = closure(&["/nix/store/tool"]);
    let before = step_key(StepKeyRequest {
        kind: "COPY",
        arguments: "COPY src .",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        predecessor: "previous",
        declared_sources: &["sha256-source-one".into()],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &[],
    })
    .unwrap();
    let after = step_key(StepKeyRequest {
        kind: "COPY",
        arguments: "COPY src .",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        predecessor: "previous",
        declared_sources: &["sha256-source-two".into()],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &[],
    })
    .unwrap();
    assert_ne!(before, after);
    assert_ne!(
        top_fetch_chain_key("fetch", &offered, &environment, "sha256-one", &[]).unwrap(),
        top_fetch_chain_key("fetch", &offered, &environment, "sha256-two", &[]).unwrap()
    );
}

#[test]
fn copy_key_arguments_exclude_physical_directive_provenance() {
    let original = Copy {
        src: Template {
            parts: vec![
                TemplatePart::Binder {
                    name: "src".into(),
                    line: 8,
                },
                TemplatePart::Literal("/rust/".into()),
            ],
        },
        dst: ".".into(),
        mode: crate::CopyMode::Materialize,
        line: 8,
        source: "COPY ${src}/rust/ .".into(),
    };
    let formatted = Copy {
        src: Template {
            parts: vec![
                TemplatePart::Binder {
                    name: "src".into(),
                    line: 7,
                },
                TemplatePart::Literal("/rust/".into()),
            ],
        },
        dst: ".".into(),
        mode: crate::CopyMode::Materialize,
        line: 7,
        source: "  COPY ${src}/rust/ .".into(),
    };

    assert_eq!(
        copy_key_arguments(&original).unwrap(),
        copy_key_arguments(&formatted).unwrap()
    );
}

#[test]
fn ordered_imports_participate_in_chain_keys() {
    let environment = BTreeMap::from([("PATH".into(), "/bin".into())]);
    let offered = closure(&["/nix/store/one", "/nix/store/two"]);
    let one_first = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "tool",
        offered_closure: &offered,
        ordered_imports: &["/nix/store/one".into(), "/nix/store/two".into()],
        predecessor: "previous",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &[],
    })
    .unwrap();
    let two_first = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "tool",
        offered_closure: &offered,
        ordered_imports: &["/nix/store/two".into(), "/nix/store/one".into()],
        predecessor: "previous",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &[],
    })
    .unwrap();
    assert_ne!(one_first, two_first);
}

#[test]
fn ordered_overlay_identity_participates_in_chain_keys() {
    let environment = BTreeMap::new();
    let base = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "true",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        predecessor: "previous",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &["base:one:overlay:a".into()],
    })
    .unwrap();
    let changed_overlay = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "true",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        predecessor: "previous",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &["base:one:overlay:b".into()],
    })
    .unwrap();
    let moved_base = step_key(StepKeyRequest {
        kind: "RUN",
        arguments: "true",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        predecessor: "previous",
        declared_sources: &[],
        environment: &environment,
        fetch_pin: None,
        universe_identities: &["base:two:overlay:a".into()],
    })
    .unwrap();
    assert_ne!(base, changed_overlay);
    assert_ne!(base, moved_base);

    let memo_base = step_memo_key(StepMemoKeyRequest {
        builder: "build",
        index: 0,
        kind: "RUN",
        directive: "RUN true",
        arguments: "true",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        environment: &environment,
        universe_identities: &["base:one:overlay:a".into()],
    })
    .unwrap();
    let memo_changed_overlay = step_memo_key(StepMemoKeyRequest {
        builder: "build",
        index: 0,
        kind: "RUN",
        directive: "RUN true",
        arguments: "true",
        offered_closure: &BTreeSet::new(),
        ordered_imports: &[],
        environment: &environment,
        universe_identities: &["base:one:overlay:b".into()],
    })
    .unwrap();
    assert_ne!(memo_base, memo_changed_overlay);
}

#[test]
fn import_union_merges_subtrees_and_preserves_earlier_collisions() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first");
    let second = root.path().join("second");
    for package in [&first, &second] {
        fs::create_dir_all(package.join("bin")).unwrap();
        fs::create_dir_all(package.join("etc/tool")).unwrap();
        fs::create_dir_all(package.join("share/tool")).unwrap();
    }
    fs::write(first.join("bin/collision"), "first").unwrap();
    fs::write(second.join("bin/collision"), "second").unwrap();
    fs::write(first.join("etc/tool/first"), "first").unwrap();
    fs::write(second.join("etc/tool/second"), "second").unwrap();
    fs::write(second.join("share/tool/data"), "shared").unwrap();

    let union = prepare_import_union(
        &[
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ],
        false,
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(union.path().join("bin/collision")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(union.path().join("etc/tool/first")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(union.path().join("etc/tool/second")).unwrap(),
        "second"
    );
    assert_eq!(
        fs::read_to_string(union.path().join("share/tool/data")).unwrap(),
        "shared"
    );
}

#[test]
fn cold_mismatch_names_the_exact_consuming_copy() {
    let warm = memo_entry(BTreeMap::from([(
        "target/release/app".into(),
        ConsumedPath {
            nar_hash: "sha256-warm".into(),
            store_path: "/nix/store/warm".into(),
        },
    )]));
    let cold = BTreeMap::from([(
        "target/release/app".into(),
        ConsumedPath {
            nar_hash: "sha256-cold".into(),
            store_path: "/nix/store/cold".into(),
        },
    )]);
    let needed = BTreeMap::from([(
        "target/release/app".into(),
        NeededPath {
            attributions: vec![Attribution {
                binder: "build".into(),
                path: "target/release/app".into(),
                line: 17,
            }],
        },
    )]);
    let error = compare_cold_paths(Some(&warm), &cold, &needed)
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "COPY ${build}/target/release/app (line 17) differs between warm and cold"
    );
}

#[test]
fn fetch_self_read_requires_the_complete_recorded_write_set() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = root.path().join("snapshot");
    let workspace = root.path().join("workspace");
    fs::create_dir_all(&snapshot).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    fs::write(snapshot.join("foo"), "first").unwrap();
    fs::write(snapshot.join("bar"), "second").unwrap();
    fs::write(workspace.join("foo"), "first").unwrap();
    let memo = StepMemo {
        key: "fetch-a".into(),
        reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
        output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
        changes: BTreeMap::from([
            ("foo".into(), StepChange::Present),
            ("bar".into(), StepChange::Present),
        ]),
        output_hashes: memo_output_hashes(
            &snapshot,
            &BTreeMap::from([
                ("foo".into(), StepChange::Present),
                ("bar".into(), StepChange::Present),
            ]),
        )
        .unwrap(),
    };

    assert!(!validate_step_memo(&memo, &workspace, true, None).unwrap().0);
    assert!(verify_cold_read_set(&memo, &workspace, 1, "FETCH a").is_err());

    fs::write(workspace.join("bar"), "second").unwrap();
    assert!(validate_step_memo(&memo, &workspace, true, None).unwrap().0);
    assert!(
        !validate_step_memo(&memo, &workspace, false, None)
            .unwrap()
            .0
    );

    fs::write(workspace.join("foo"), "drifted").unwrap();
    assert!(!validate_step_memo(&memo, &workspace, true, None).unwrap().0);
}

#[test]
fn self_read_exception_never_crosses_memo_owners() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = root.path().join("snapshot");
    let workspace = root.path().join("workspace");
    fs::create_dir_all(&snapshot).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    fs::write(snapshot.join("foo"), "a-output").unwrap();
    fs::write(workspace.join("foo"), "a-output").unwrap();
    let a = StepMemo {
        key: "fetch-a".into(),
        reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
        output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
        changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
        output_hashes: memo_output_hashes(
            &snapshot,
            &BTreeMap::from([("foo".into(), StepChange::Present)]),
        )
        .unwrap(),
    };
    let b = StepMemo {
        key: "run-b".into(),
        reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
        output_snapshot: None,
        changes: BTreeMap::new(),
        output_hashes: BTreeMap::new(),
    };

    assert!(validate_step_memo(&a, &workspace, true, None).unwrap().0);
    assert!(!validate_step_memo(&b, &workspace, true, None).unwrap().0);
}

#[test]
fn a_fetch_self_states_never_allow_b_to_bypass_its_own_fingerprint() {
    let root = tempfile::tempdir().unwrap();
    let snapshot = root.path().join("snapshot");
    let workspace = root.path().join("workspace");
    fs::create_dir_all(&snapshot).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    fs::write(snapshot.join("foo"), "pinned-a-output").unwrap();
    fs::write(workspace.join("foo"), "pinned-a-output").unwrap();
    let a = StepMemo {
        key: "fetch-a".into(),
        reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
        output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
        changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
        output_hashes: memo_output_hashes(
            &snapshot,
            &BTreeMap::from([("foo".into(), StepChange::Present)]),
        )
        .unwrap(),
    };
    let b = StepMemo {
        key: "run-b".into(),
        reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
        output_snapshot: None,
        changes: BTreeMap::new(),
        output_hashes: BTreeMap::new(),
    };

    // a may use its own constructive output; b's read is still checked
    // only against b's recorded fingerprint.
    assert!(validate_step_memo(&a, &workspace, true, None).unwrap().0);
    assert!(!validate_step_memo(&b, &workspace, false, None).unwrap().0);

    // If a executes again and its output moves, b remains a miss and the
    // automatic FETCH pin stays the loud boundary until --update-lock.
    revert_step_writes(&a, &workspace).unwrap();
    fs::write(workspace.join("foo"), "drifted-a-output").unwrap();
    assert!(!validate_step_memo(&a, &workspace, true, None).unwrap().0);
    assert!(!validate_step_memo(&b, &workspace, false, None).unwrap().0);
    let error = FetchState::verify(
        None,
        Some(&FetchPin::expected("sha256-pinned".into())),
        Some("sha256-drifted"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("--update-lock"), "{error}");
}

#[test]
fn executing_fetch_reverts_its_superseded_writes_before_tracing() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("foo"), "old fetch output").unwrap();
    fs::write(root.path().join("kept"), "not fetch-owned").unwrap();
    let memo = StepMemo {
        key: "fetch-a".into(),
        reads: BTreeMap::new(),
        output_snapshot: None,
        changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
        output_hashes: BTreeMap::new(),
    };

    revert_step_writes(&memo, root.path()).unwrap();
    assert!(!root.path().join("foo").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("kept")).unwrap(),
        "not fetch-owned"
    );
}

#[test]
fn fetch_records_a_new_output_tree_as_one_constructive_root() {
    let before = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::create_dir_all(workspace.path().join("vendor/nested")).unwrap();
    fs::write(workspace.path().join("vendor/first"), "one").unwrap();
    fs::write(workspace.path().join("vendor/nested/second"), "two").unwrap();
    let mut changes = BTreeMap::from([
        ("vendor/first".into(), StepChange::Present),
        ("vendor/nested/second".into(), StepChange::Present),
    ]);

    retain_fetch_output_roots(before.path(), workspace.path(), &mut changes).unwrap();
    assert_eq!(
        changes,
        BTreeMap::from([("vendor".into(), StepChange::Present)])
    );
}

#[test]
fn first_staging_overrides_prior_step_output_then_preserves_upper_writes() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("work");
    let baseline = root.path().join("staged/step");
    let source = root.path().join("source");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(workspace.join("value"), "earlier command").unwrap();
    fs::write(&source, "declared v1").unwrap();

    stage_input(&source, "value", &workspace, &baseline).unwrap();
    assert_eq!(
        fs::read_to_string(workspace.join("value")).unwrap(),
        "declared v1"
    );

    fs::write(workspace.join("value"), "later command").unwrap();
    fs::write(&source, "declared v2").unwrap();
    stage_input(&source, "value", &workspace, &baseline).unwrap();
    assert_eq!(
        fs::read_to_string(workspace.join("value")).unwrap(),
        "later command"
    );
}

#[test]
fn fetch_pin_mismatch_is_loud_and_names_update_lock() {
    let error = FetchState::verify(
        None,
        Some(&FetchPin::expected("sha256-old".into())),
        Some("sha256-new"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("hash mismatch"), "{error}");
    assert!(error.contains("--update-lock"), "{error}");
}

#[test]
fn volatile_facts_follow_only_consumed_path_boundaries() {
    let observed = BTreeMap::from([
        (
            ".npm/_logs/timestamped-debug.log".into(),
            VolatilePath {
                first_size: 1,
                second_size: 2,
            },
        ),
        (
            "node_modules/pkg/index.js".into(),
            VolatilePath {
                first_size: 3,
                second_size: 4,
            },
        ),
        (
            "result".into(),
            VolatilePath {
                first_size: 5,
                second_size: 6,
            },
        ),
    ]);
    let needed = BTreeMap::from([
        ("node_modules".into(), NeededPath::default()),
        ("result".into(), NeededPath::default()),
    ]);

    assert_eq!(
        FetchState::consumed_volatility(observed, &needed)
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["node_modules/pkg/index.js", "result"]
    );
}

#[test]
fn persistent_workspace_identity_includes_builder_name() {
    let directory = Path::new("/work/project");
    assert_ne!(
        workspace_identity(directory, "frontend"),
        workspace_identity(directory, "backend")
    );
    assert_eq!(
        workspace_identity(directory, "frontend"),
        workspace_identity(directory, "frontend")
    );
}

#[test]
fn socket_filter_failure_adds_localhost_hint() {
    let error = sandbox_failure("exit status: 1", Some(RunNetwork::SocketFilter));
    assert!(error.contains("sandboxing was not weakened"), "{error}");
    assert!(error.contains("socket-filter fallback"), "{error}");
    assert!(
        error.contains("localhost networking (127.0.0.1) was unavailable"),
        "{error}"
    );
    assert_eq!(error.lines().count(), 2, "{error}");

    let preferred = sandbox_failure("exit status: 1", Some(RunNetwork::Namespace));
    assert!(!preferred.contains("localhost"), "{preferred}");
}

#[test]
fn failure_hints_require_exact_tls_and_pnpm_evidence() {
    let certificate_tail = r#"
42 newfstatat(AT_FDCWD, "/nix/store/openssl/etc/ssl/certs/5ad8a5d6.0", 0x0, 0) = -1 ENOENT
42 newfstatat(AT_FDCWD, "/nix/store/openssl/etc/ssl/certs/919ba934.0", 0x0, 0) = -1 ENOENT
42 newfstatat(AT_FDCWD, "/nix/store/openssl/etc/ssl/certs/a3418fda.0", 0x0, 0) = -1 ENOENT
"#;
    assert_eq!(
        failure_problem_hints(Some(124), true, "", "", Some(certificate_tail)),
        ["hint: TLS-trust masquerade: this FETCH timed out after repeated failed certificate probes; IMPORT ${pkgs.cacert} (or another declared CA bundle); see docs/cixfile.md#fetch-tls-trust"]
    );
    assert!(failure_problem_hints(
        Some(124),
        true,
        "",
        "unrelated timeout",
        Some("42 openat(AT_FDCWD, \"/tmp/certificate\", O_RDONLY) = -1 ENOENT")
    )
    .is_empty());
    assert!(failure_problem_hints(Some(1), true, "", "", Some(certificate_tail)).is_empty());

    assert_eq!(
        failure_problem_hints(
            Some(1),
            false,
            "ERR_PNPM_NO_OFFLINE_TARBALL missing package",
            "",
            None
        ),
        ["hint: pnpm offline/store wall: seal the complete fetched store and install with frozen-store=true, --offline, and --frozen-lockfile using pnpm >=11.7 and Node >=22.15; see docs/cixfile.md#pnpm-frozen-store"]
    );
    assert_eq!(
        failure_problem_hints(
            Some(1),
            false,
            "",
            "ERR_PNPM_FROZEN_STORE_UNSUPPORTED_NODE",
            None
        ),
        ["hint: pnpm offline/store wall: seal the complete fetched store and install with frozen-store=true, --offline, and --frozen-lockfile using pnpm >=11.7 and Node >=22.15; see docs/cixfile.md#pnpm-frozen-store"]
    );
    assert!(failure_problem_hints(
        Some(1),
        false,
        "offline build failed",
        "readonly database",
        None
    )
    .is_empty());
}

#[test]
fn socket_filter_is_accepted_by_bubblewrap() {
    let shell = fs::read_dir("/nix/store")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin/bash"))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
        .expect("the Nix test host provides a resolvable bash");
    let offer = shell
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let offered_closure = NixEvaluation::offered_closure(std::slice::from_ref(&offer)).unwrap();
    let work = tempfile::tempdir().unwrap();
    let command = ResolvedCommand::Legacy {
        command: "printf fallback-ok > result".into(),
    };

    Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command: &command,
        environment: &BTreeMap::new(),
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: &[offer],
        run_network: Some(RunNetwork::SocketFilter),
        credentials: &[],
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(work.path().join("result")).unwrap(),
        "fallback-ok"
    );
}

#[test]
fn argv_and_heredoc_cross_the_sandbox_as_declared() {
    let package_with = |binary: &str| {
        fs::read_dir("/nix/store")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("bin").join(binary))
            .filter(|candidate| candidate.is_file())
            .filter_map(|candidate| candidate.canonicalize().ok())
            .map(|resolved| resolved.parent().unwrap().parent().unwrap().to_path_buf())
            .find(|package| package.join("bin").join(binary).is_file())
            .unwrap_or_else(|| panic!("the Nix test host provides a resolvable {binary}"))
            .to_string_lossy()
            .into_owned()
    };
    let bash = package_with("bash");
    let coreutils = package_with("touch");
    let imports = vec![bash, coreutils];
    let offered_closure = NixEvaluation::offered_closure(&imports).unwrap();
    let work = tempfile::tempdir().unwrap();
    let argv = ResolvedCommand::Argv {
        argv: vec!["touch".into(), "literal;touch injected".into()],
    };

    Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command: &argv,
        environment: &BTreeMap::new(),
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: &imports,
        run_network: None,
        credentials: &[],
    })
    .unwrap();
    assert!(work.path().join("literal;touch injected").is_file());
    assert!(!work.path().join("injected").exists());

    let heredoc = ResolvedCommand::Heredoc {
        interpreter: "bash".into(),
        body: "printf '%s' \"$0\" > heredoc-filename\nprintf body-ok > heredoc-result\n".into(),
    };
    Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command: &heredoc,
        environment: &BTreeMap::new(),
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: &imports,
        run_network: None,
        credentials: &[],
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(work.path().join("heredoc-filename")).unwrap(),
        "/run/cix-heredoc"
    );
    assert_eq!(
        fs::read_to_string(work.path().join("heredoc-result")).unwrap(),
        "body-ok"
    );
}

#[test]
fn node_environment_does_not_leak_between_sandbox_calls() {
    let bash = fs::read_dir("/nix/store")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin/bash"))
        .filter(|candidate| candidate.is_file())
        .filter_map(|candidate| candidate.canonicalize().ok())
        .map(|resolved| resolved.parent().unwrap().parent().unwrap().to_path_buf())
        .find(|package| package.join("bin/bash").is_file())
        .expect("the Nix test host provides a resolvable bash")
        .to_string_lossy()
        .into_owned();
    let offered_closure = NixEvaluation::offered_closure(std::slice::from_ref(&bash)).unwrap();
    let work = tempfile::tempdir().unwrap();
    let first = ResolvedCommand::Argv {
        argv: vec![
            "bash".into(),
            "-c".into(),
            "printf '%s' \"$SCOPED\" > first-env".into(),
        ],
    };
    let second = ResolvedCommand::Argv {
        argv: vec![
            "bash".into(),
            "-c".into(),
            "printf '%s' \"${SCOPED-unset}\" > second-env".into(),
        ],
    };
    Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command: &first,
        environment: &BTreeMap::from([("SCOPED".into(), "first-only".into())]),
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: std::slice::from_ref(&bash),
        run_network: None,
        credentials: &[],
    })
    .unwrap();
    Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command: &second,
        environment: &BTreeMap::new(),
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: std::slice::from_ref(&bash),
        run_network: None,
        credentials: &[],
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(work.path().join("first-env")).unwrap(),
        "first-only"
    );
    assert_eq!(
        fs::read_to_string(work.path().join("second-env")).unwrap(),
        "unset"
    );
}

#[test]
fn fetch_credential_matching_uses_concrete_url_prefixes() {
    assert_eq!(
        url_prefix("https://packages.example.test/team/npm/pkg.tgz").unwrap(),
        "https://packages.example.test/team"
    );
    assert!(token_matches(
        "https://packages.example.test/team/*",
        "https://packages.example.test/team/npm/pkg.tgz"
    ));
    assert!(!token_matches(
        "https://packages.example.test/other/*",
        "https://packages.example.test/team/npm/pkg.tgz"
    ));
    assert_eq!(
        concrete_fetch_url("curl --fail 'https://packages.example.test/team/npm/pkg.tgz'"),
        Some("https://packages.example.test/team/npm/pkg.tgz".into())
    );
}

#[test]
fn fetch_consent_is_scoped_to_project_prefix_and_token() {
    let project = PathBuf::from("/work/example");
    let first = Consent {
        project: project.clone(),
        token: "packages".into(),
        prefix: "https://packages.example.test/team".into(),
    };
    let second_prefix = Consent {
        project: project.clone(),
        token: "packages".into(),
        prefix: "https://packages.example.test/other".into(),
    };
    let other_project = Consent {
        project: PathBuf::from("/work/other"),
        token: "packages".into(),
        prefix: first.prefix.clone(),
    };
    let mut store = ConsentStore {
        grants: BTreeSet::from([first.clone(), second_prefix, other_project]),
    };

    assert!(store.grants.contains(&first));
    assert_eq!(revoke_from_store(&mut store, "packages"), 3);
    assert!(store.grants.is_empty());
}

#[test]
fn removed_fetch_token_refuses_an_anonymous_retry() {
    let project = PathBuf::from("/work/example");
    let mut credentials = HostCredentials {
        project: project.clone(),
        tokens: BTreeMap::new(),
        consent_path: PathBuf::from("/tmp/fetch-consents.json"),
        consent: ConsentStore {
            grants: BTreeSet::from([Consent {
                project,
                token: "retired".into(),
                prefix: "https://packages.example.test/team".into(),
            }]),
        },
        allow_secret: true,
    };

    let error = match credentials.for_command("curl https://packages.example.test/team/pkg.tgz") {
        Ok(_) => panic!("a removed token must not allow an anonymous FETCH"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("retired"), "{error}");
    assert!(error.contains("refusing anonymous retry"), "{error}");
}

#[test]
fn a_new_fetch_prefix_needs_its_own_consent() {
    let directory = tempfile::tempdir().unwrap();
    let credential = directory.path().join("credential");
    fs::write(&credential, "not logged").unwrap();
    let project = PathBuf::from("/work/example");
    let old = Consent {
        project: project.clone(),
        token: "packages".into(),
        prefix: "https://packages.example.test/team".into(),
    };
    let mut credentials = HostCredentials {
        project: project.clone(),
        tokens: BTreeMap::from([(
            "packages".into(),
            CredentialToken {
                url: "https://packages.example.test/*".into(),
                credential,
            },
        )]),
        consent_path: directory.path().join("consent.json"),
        consent: ConsentStore {
            grants: BTreeSet::from([old]),
        },
        allow_secret: true,
    };

    let mounted = credentials
        .for_command("curl https://packages.example.test/other/pkg.tgz")
        .unwrap()
        .expect("matching token is available");
    assert_eq!(mounted.name, "packages");
    assert!(credentials
        .consent
        .grants
        .iter()
        .all(|grant| grant.prefix != "https://packages.example.test/other"));
}
