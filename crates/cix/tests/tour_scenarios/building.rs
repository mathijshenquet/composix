use super::super::*;

pub(crate) fn chapter_building() -> String {
    let mut doc = Doc::new("building");

    let fetch_demo = doc.base.join("fetch-demo");
    fs::create_dir(&fetch_demo).expect("creating FETCH demo");
    let expected_hash = tree_hash("expected", b"author-pinned");
    fs::write(
        fetch_demo.join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH expected ${{pkgs.coreutils}}/bin/printf author-pinned > expected EXPECT {expected_hash}
FETCH resolved ${{pkgs.coreutils}}/bin/printf lock-pinned > resolved

BUILDER assemble
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{expected}}/expected expected
COPY ${{resolved}}/resolved resolved
RUN cat expected resolved > result

ITEM fetched-result
COPY ${{assemble}}/result /result
"#
        ),
    )
    .expect("writing FETCH demo Cixfile");
    fs::write(fetch_demo.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing FETCH demo lock");

    doc.para("You will pin downloaded inputs, reuse checked build work, replay it from an empty workspace, repair a conventional Linux binary, and compile a two-service Rust project. A **lock** is the checked-in `Cixfile.lock` beside each Cixfile; it records immutable source revisions, download pins, and the evidence needed to validate reusable build steps.");

    doc.para("## FETCH, EXPECT, and deliberate lock movement");
    doc.para("Choose `EXPECT` when an author or upstream release gives you a trusted checksum. Its SRI value is a `sha256-…` integrity fingerprint over the complete serialized output directory; to calculate one for a download you have independently inspected, put the fetched files in one directory and run `nix hash path --sri that-directory`. Without `EXPECT`, `--update-lock` uses trust on first use: cix fetches twice to expose immediate volatility and pins only paths used downstream, but two matching responses do not authenticate a consistently malicious server.");
    doc.para("Read the first line as labelled grammar: in `FETCH expected ${pkgs.coreutils}/bin/printf author-pinned > expected EXPECT sha256-…`, the first `expected` is the new binder name, everything through `> expected` is the network-enabled command and its declared output file, and the final `EXPECT` clause is the author-supplied whole-tree hash. The second FETCH omits that clause, so only an explicit update may create or change its lock pin.");
    let fetch_source = doc.show_file("fetch-demo/Cixfile");
    assert!(fetch_source.contains("FETCH expected"));
    assert!(fetch_source.contains("EXPECT sha256-"));
    assert!(fetch_source.contains("FETCH resolved"));
    assert!(fetch_source.contains("RUN cat expected resolved > result"));

    doc.para("The tour sets `CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces` only to keep its disposable work below this fixture. You do not need to create it: cix does so automatically, and without the variable it uses the user cache at `~/.cache/cix/workspaces`.");
    doc.para("A **memo** is a recorded build-step result keyed by the command, imports, environment, and observed inputs. A **build view** is the immutable `/nix/store` snapshot of the memo's output that later COPY steps consume; a miss executes the step and writes a view, while a hit reuses the prior view after rechecking its inputs. `--update-lock resolved` permits the network for that named FETCH, rewrites `fetch-demo/Cixfile.lock`, and should be followed by committing that lock.");
    let updated = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --update-lock resolved fetch-demo",
        true,
    );
    assert!(updated.contains("BUILDER assemble memo miss"), "{updated}");
    let result_path = built_store_path(&updated, "-cix-item-fetched-result");
    let combined = doc.show_file(Path::new(&result_path).join("result"));
    assert_eq!(combined.trim(), "author-pinnedlock-pinned");

    doc.para("The lock is written beside the Cixfile, not in the workspace. It keeps the immutable nixpkgs revision, each FETCH pin, step memos, consumed output objects, and a **development-environment snapshot**: the complete set of environment variables derived together from one builder's imported package universe. That is why compiler-related values such as `PKG_CONFIG_PATH` arrive as one pinned set instead of hand-wired host paths; inspect the full records with `jq '.devEnvs' fetch-demo/Cixfile.lock`.");
    let lock = doc.sh(
        "jq '{fetches, devEnvCount:(.devEnvs | length)}' fetch-demo/Cixfile.lock",
        true,
    );
    assert!(lock.contains("\"expected\""));
    assert!(lock.contains("\"resolved\""));
    assert!(lock.contains("\"devEnvCount\": 1"));

    doc.para("FETCH alone has network authority. RUN executes in a bubblewrap sandbox: a temporary filesystem and namespace containing only the declared packages and workspace, with networking removed. Cix follows the command and all subprocesses with `strace` until they exit, recording file opens, metadata checks, directory listings, missing paths, and writes; the command, imports, environment, and that observed read set form the reusable step identity.");
    let warm = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --stats fetch-demo",
        true,
    );
    assert!(warm.contains("memo-hit"), "{warm}");
    assert!(warm.contains("\"nixSubprocesses\": 0"), "{warm}");
    doc.para("That hit is not a timestamp promise. For example, `RUN cat expected resolved > result` records reads of those two files: changing either produces a miss, while adding an unread `notes` file does not. Directory enumeration, metadata-only probes, and an absent file are fingerprinted too; nondeterministic reads therefore cause a miss or a warm-versus-cold audit error instead of becoming invisible state. A persistent workspace is an acceleration structure, not hidden build input.");

    doc.para("`--update-lock` and `--cold` are the audit pair. The first permits the network, writes the pin to `Cixfile.lock`, stores fetched bytes as a Nix store snapshot, and records a receipt below `~/.cache/cix/fetch-snapshots`. `--cold` creates an empty builder workspace, never contacts the network, replays those pinned bytes from the local snapshot cache, and compares the new reads and outputs. A different machine must first perform an ordinary pin-verifying build or receive that cached store closure; if the receipt or store snapshot was garbage-collected, cold replay refuses and tells you to repopulate it rather than refetching silently.");
    let cold = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --cold fetch-demo",
        true,
    );
    assert!(cold.contains("BUILDER assemble"), "{cold}");
    assert_eq!(
        built_store_path(&cold, "-cix-item-fetched-result"),
        result_path
    );

    doc.para("## The FHS diagnostic, then the one-line fix");
    let fhs_fixture = fhs_elf_fixture();
    let fhs_bytes = fs::read(fhs_fixture.join("fhs-probe")).expect("reading FHS probe");
    let fhs_hash = tree_hash("fhs-probe", &fhs_bytes);
    let listen = next_listen();
    let _server = start_file_server(&doc, &fhs_fixture, &listen);
    let fhs_demo = doc.base.join("fhs-demo");
    fs::create_dir(&fhs_demo).expect("creating FHS demo");
    fs::write(
        fhs_demo.join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH native ${{pkgs.curl}}/bin/curl -fsS http://{listen}/fhs-probe -o fhs-probe EXPECT {fhs_hash}

BUILDER native-build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{native}}/fhs-probe .
RUN chmod +x fhs-probe
RUN ./fhs-probe > result

ITEM native-result
COPY ${{native-build}}/result /result
"#
        ),
    )
    .expect("writing FHS demo Cixfile");
    fs::write(fhs_demo.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing FHS demo lock");

    doc.para("The tour harness serves the next fixture URL from a temporary local HTTP server; in your Cixfile substitute any real URL and its independently obtained EXPECT hash. The downloaded ELF is a conventional Linux executable whose header demands the fixed loader path `/lib64/ld-linux-x86-64.so.2`. Nix normally keeps that loader under glibc's unique store path, so merely having the executable bytes is insufficient. The builder imports a shell and core utilities but no libc, and the real trace produces the failure below.");
    let fhs_source = doc.show_file("fhs-demo/Cixfile");
    assert!(fhs_source.contains("http://"));
    assert!(!fhs_source.contains("pkgs.glibc"));
    let missing = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo",
        false,
    );
    assert!(
        missing.contains("fhs-probe requires the FHS loader"),
        "{missing}"
    );
    assert!(missing.contains("/lib64/ld-linux-x86-64.so.2"), "{missing}");
    assert!(missing.contains("IMPORT ${pkgs.glibc}"), "{missing}");

    doc.para("Add glibc to the ordered IMPORT union. In the builder sandbox, that import mounts glibc's loader at the conventional `/lib64/ld-linux-x86-64.so.2` alias and offers its library closure; the same bytes now run without mutation or a patchelf step.");
    doc.sh(
        "sed -i 's/${pkgs.coreutils}/${pkgs.coreutils} ${pkgs.glibc}/' fhs-demo/Cixfile",
        true,
    );
    let repaired_source = doc.sh("grep '^IMPORT' fhs-demo/Cixfile", true);
    assert!(repaired_source.contains("${pkgs.glibc}"));
    let repaired = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo",
        true,
    );
    let repaired_path = built_store_path(&repaired, "-cix-item-native-result");
    let fhs_result = doc.show_file(Path::new(&repaired_path).join("result"));
    assert_eq!(fhs_result.trim(), "fhs-tour-ok");

    doc.para("## Capstone: one Rust workspace, two services");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/build/proj1");
    for relative in PROJ1_FILES {
        let destination = doc.base.join("proj1").join(relative);
        fs::create_dir_all(destination.parent().expect("project file has a parent"))
            .expect("creating proj1 directory");
        fs::copy(source.join(relative), destination).expect("copying proj1 fixture");
    }
    doc.para("The capstone is a complete small Cargo workspace copied from this repository's `examples/build/proj1`. `Cargo.toml` declares the three local members, `Cargo.lock` pins their dependency graph, and the source tree is shown below. There are no registry dependencies in this fixture, so `--offline` needs no unseen vendor directory; cargo, rustc, gcc, and coreutils come from the nixpkgs revision shown in `Cixfile.lock`.");
    let proj1_source = [
        "proj1/Cixfile",
        "proj1/Cixfile.lock",
        "proj1/rust/Cargo.toml",
        "proj1/rust/Cargo.lock",
    ]
    .map(|path| doc.show_file(path))
    .join("");
    assert!(proj1_source.contains("cargo build --release --locked --offline --workspace"));
    assert!(proj1_source.contains("SERVICE proj1-api"));
    assert!(proj1_source.contains("SERVICE proj1-worker"));
    let layout = doc.sh(
        "find proj1/rust -type f -not -path '*/target/*' | sort",
        true,
    );
    assert!(layout.contains("proj1/rust/api/src/main.rs"));
    assert!(layout.contains("proj1/rust/worker/src/main.rs"));
    assert!(layout.contains("proj1/rust/common/src/lib.rs"));

    doc.para("The positional `proj1` chooses that directory. `--namespace proj1` supplies the slash-grouped tag family, and `-t v1` tags both declared members, yielding `proj1/proj1-api:v1` and `proj1/proj1-worker:v1`. A `directory#member` selector instead builds one final member and its backward dependency slice; it cannot be combined with family tagging.");
    let first = doc.sh(
        "items=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1 --namespace proj1 -t v1); api_v1=$(printf '%s\\n' \"$items\" | jq -r '.\"proj1-api\"'); worker_v1=$(printf '%s\\n' \"$items\" | jq -r '.\"proj1-worker\"'); printf '%s\\n' \"$items\"",
        true,
    );
    let first_api = proj1_item_path(&first, "proj1-api");
    let first_worker = proj1_item_path(&first, "proj1-worker");
    let refs = doc.sh("cix ls proj1/", true);
    assert!(refs.contains("proj1/proj1-api:v1"), "{refs}");
    assert!(refs.contains("proj1/proj1-worker:v1"), "{refs}");
    let before_worker = doc.sh_with_env(
        "\"$worker_v1/bin/proj1-worker\"",
        &[("worker_v1", &first_worker)],
        true,
    );
    assert_eq!(before_worker.trim(), "hello from proj1-worker");

    doc.para("Now change only the worker and select that member. Cargo reruns because the shared builder staged the changed workspace, but each final SERVICE depends only on the release binary it copied from that workspace.");
    doc.sh(
        "sed -i 's/proj1-worker/proj1-worker-edited/' proj1/rust/worker/src/main.rs",
        true,
    );
    let edited_worker = doc.sh(
        "worker_v2=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-worker); printf '%s\\n' \"$worker_v2\"",
        true,
    );
    let edited_worker = edited_worker
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected worker build printed its item")
        .to_owned();
    assert_ne!(edited_worker, first_worker);
    let worker_receipt = doc.sh_with_env(
        "\"$worker_v2/bin/proj1-worker\"",
        &[("worker_v2", &edited_worker)],
        true,
    );
    assert_eq!(worker_receipt.trim(), "hello from proj1-worker-edited");
    let unchanged_api = doc.sh(
        "api_after=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-api); printf '%s\\n' \"$api_after\"",
        true,
    );
    let unchanged_api = unchanged_api
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected API build printed its item")
        .to_owned();
    assert_eq!(unchanged_api, first_api);
    let api_receipt = doc.sh_with_env(
        "cmp -s \"$api_v1\" \"$api_after\" && printf 'API item reused byte-for-byte\\n'",
        &[("api_v1", &first_api), ("api_after", &unchanged_api)],
        true,
    );
    assert_eq!(api_receipt.trim(), "API item reused byte-for-byte");
    doc.para("The worker receipt visibly changes while the API item is byte-for-byte identical. The warm workspace and memo records stay private to the builder—they are acceleration state, not runtime dependencies—while each final item's Nix closure contains only the immutable paths reached by what that member copied.");
    doc.finish()
}
