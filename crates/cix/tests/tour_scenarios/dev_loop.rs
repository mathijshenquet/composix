use super::super::*;

pub(crate) fn chapter_dev_loop() -> String {
    let mut doc = Doc::new("dev-loop");

    doc.para("You will rebuild an immutable item after a source edit, stop the watcher cleanly, and compare two runnable Cixfiles derived from the same five-line Dockerfile. A faithful translation preserves the source build's observable steps and process interface; a dissolved translation uses a Nix package directly when those steps add no behavior. The explanation stands on files, build outputs, and systemd processes; Docker familiarity is optional.");

    doc.para("## Rebuild an item after a file changes");
    doc.para("`cix watch PATH` requires the same Nix/flakes setup as `cix build`. It recursively observes PATH, maps an edited file to the nearest containing Cixfile, rebuilds that artifact with cached builder state, and prints each new immutable store path. A bare watch does not tag the item and does not run it.");
    let cache_root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME is set for the tour")
        .join(".cache/cix/tmp");
    fs::create_dir_all(&cache_root).expect("creating cix cache temp root");
    let watch_temp = tempfile::Builder::new()
        .prefix("tour-watch-")
        .tempdir_in(&cache_root)
        .expect("creating watch fixture outside ignored target paths");
    let watch_dir = watch_temp.path().to_owned();
    fs::create_dir(watch_dir.join(".git")).expect("marking watch fixture as its own repository");
    std::os::unix::fs::symlink(&watch_dir, doc.base.join("watch-app"))
        .expect("linking watch fixture into the tour story");
    fs::write(watch_dir.join("message"), "first\n").expect("writing watch message");
    fs::write(
        watch_dir.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

ITEM watched
COPY message /message
"#,
    )
    .expect("writing watch Cixfile");
    fs::write(watch_dir.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing watch lock");
    let watch_source = ["watch-app/Cixfile", "watch-app/message"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(watch_source.contains("ITEM watched"));
    assert!(watch_source.contains("first"));

    let mut path = doc.bin_dir.display().to_string();
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(':');
        path.push_str(&existing.to_string_lossy());
    }
    let mut watcher = Command::new(doc.bin_dir.join("cix"))
        .args(["watch", "watch-app"])
        .current_dir(&doc.base)
        .env("CIX_STATE_DIR", &doc.state_dir)
        .env(
            "CIX_BUILD_WORKSPACE_DIR",
            doc.base.join(".watch-workspaces"),
        )
        .env("CIX_WATCH_DEBOUNCE_MS", "30")
        .env("PATH", path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("starting cix watch");
    let stdout = watcher.stdout.take().expect("watch stdout");
    let (lines_sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let _ = lines_sender.send(line.expect("watch stdout line"));
        }
    });
    let stderr = watcher.stderr.take().expect("watch stderr");
    let (errors_sender, errors) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = errors_sender.send(line.expect("watch stderr line"));
        }
    });
    let watching = errors
        .recv_timeout(Duration::from_secs(5))
        .expect("watcher announces its root");
    assert!(watching.starts_with("watching "));
    doc.record("$", "cix watch watch-app & watch_pid=$!", "");
    doc.output(&watching.replace(
        watch_dir.to_string_lossy().as_ref(),
        doc.base.join("watch-app").to_string_lossy().as_ref(),
    ));
    std::thread::sleep(Duration::from_millis(100));
    doc.sh("printf 'changed\\n' > watch-app/message", true);
    let rebuilt = match lines.recv_timeout(Duration::from_secs(60)) {
        Ok(line) => line,
        Err(error) => {
            unsafe {
                libc::kill(watcher.id() as i32, libc::SIGINT);
            }
            let _ = watcher.wait();
            panic!(
                "edited watch context should rebuild ({error}); watcher stderr: {:?}",
                errors.try_iter().collect::<Vec<_>>()
            );
        }
    };
    assert!(rebuilt.starts_with("/nix/store/"), "{rebuilt}");
    doc.output(&rebuilt);
    let watch_pid = watcher.id().to_string();
    doc.sh_with_env(
        "kill -INT \"$watch_pid\"",
        &[("watch_pid", &watch_pid)],
        true,
    );
    assert!(watcher.wait().expect("waiting for cix watch").success());
    assert!(errors.try_iter().next().is_none());
    drop(watch_temp);

    doc.para("The watcher coalesces edit bursts and reuses builder workspaces and FETCH memos beneath `~/.cache/cix/workspaces` by default; this generated receipt overrides that location with a throwaway directory. Reuse may speed a build but cannot change its checked immutable result. The watcher ignores `.git`, `target`, Cixfile locks, its own workspaces, and gitignored paths, so outputs do not trigger loops.");
    doc.para("When PATH contains `compose.json`, use `sudo env CIX_STATE_DIR=/var/lib/cix-index CIX_BUILD_WORKSPACE_DIR=/var/cache/cix/workspaces cix watch .`. Each edit maps to a member Cixfile, builds and retags only its corresponding child, then invokes the privileged targeted compose activation described in Chapter 6. A failed build or activation is printed as `watch round failed`; the watcher keeps listening, and a partial systemd activation has the same inspect-or-rollback boundary as an ordinary `cix up`.");

    doc.para("For source-level feedback inside one language toolchain, use a Nix development shell instead. `nix develop` realizes the tools declared by a flake and runs your command with them on PATH; it neither builds a cix item nor manages a systemd unit. Choose it for compiler, test, or framework hot-reload loops, and use `cix watch` when you need to test the immutable artifact boundary.");
    let dev_shell = doc.base.join("dev-shell");
    fs::create_dir(&dev_shell).expect("creating development-shell fixture");
    fs::write(
        dev_shell.join("flake.nix"),
        r#"{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
    in {
      devShells = nixpkgs.lib.genAttrs systems (system:
        let pkgs = import nixpkgs { inherit system; };
        in { default = pkgs.mkShell { packages = [ pkgs.hello ]; }; });
    };
}
"#,
    )
    .expect("writing development-shell flake");
    fs::write(
        dev_shell.join("flake.lock"),
        r#"{
  "nodes": {
    "nixpkgs": {
      "locked": {
        "lastModified": 1785090369,
        "narHash": "sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4=",
        "owner": "NixOS",
        "repo": "nixpkgs",
        "rev": "624af665418d3c65d544145b4d34ad696439570e",
        "type": "github"
      },
      "original": {
        "owner": "NixOS",
        "ref": "nixos-unstable",
        "repo": "nixpkgs",
        "type": "github"
      }
    },
    "root": {"inputs": {"nixpkgs": "nixpkgs"}}
  },
  "root": "root",
  "version": 7
}
"#,
    )
    .expect("writing development-shell lock");
    let shell_files = ["dev-shell/flake.nix", "dev-shell/flake.lock"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(shell_files.contains("pkgs.hello"));
    let developed = doc.sh(
        "nix develop path:./dev-shell --command sh -c 'printf \"dev shell tool: \"; hello --version | sed -n \"1p\"'",
        true,
    );
    assert!(developed.starts_with("dev shell tool: hello (GNU Hello)"));

    doc.para("## Translate one real five-line Dockerfile two ways");
    let twins = doc.base.join("twins");
    fs::create_dir(&twins).expect("creating translation twins");
    fs::write(
        twins.join("Dockerfile"),
        r#"FROM alpine:3.22
RUN apk add --no-cache hello
RUN printf '#!/bin/sh\nexec hello\n' > /usr/local/bin/start
RUN chmod +x /usr/local/bin/start
ENTRYPOINT ["/usr/local/bin/start"]
"#,
    )
    .expect("writing five-line source Dockerfile");
    fs::write(twins.join("start"), "#!/bin/bash\nexec hello\n")
        .expect("writing faithful launcher source");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(twins.join("start"))
            .expect("reading faithful launcher permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(twins.join("start"), permissions)
            .expect("making faithful launcher executable");
    }
    fs::write(
        twins.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER faithful-build
IMPORT ${pkgs.bash} ${pkgs.coreutils} ${pkgs.hello}
COPY ${src}/start .
RUN chmod +x start

APP faithful
IMPORT ${pkgs.bash} ${pkgs.hello}
COPY ${faithful-build}/start /bin/start
START start
"#,
    )
    .expect("writing faithful Cixfile");
    fs::write(twins.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing faithful lock");
    fs::write(
        twins.join("Cixfile.dissolved"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

APP dissolved
IMPORT ${pkgs.hello}
START hello
"#,
    )
    .expect("writing dissolved Cixfile");
    fs::write(twins.join("Cixfile.dissolved.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing dissolved lock");

    doc.para("The Dockerfile starts from Alpine Linux, installs GNU Hello with Alpine's package manager, writes a launcher, marks it executable, and makes that launcher the process. The faithful Cixfile keeps an explicit launcher-producing builder and the same final process interface, while obtaining Hello and Bash from Nix rather than executing `apk`. The dissolved Cixfile observes that the only runtime behavior is `hello`, imports that Nix package, and starts it directly.");
    doc.para("Use this decision rule: **is the app in nixpkgs, and is the Dockerfile only ceremony?** If both answers are yes and probes show no build-generated configuration or behavior to preserve, prefer the dissolved form. Otherwise begin faithful, verify outputs and runtime behavior, and dissolve only the steps proven irrelevant.");
    let twin_source = [
        "twins/Dockerfile",
        "twins/start",
        "twins/Cixfile",
        "twins/Cixfile.dissolved",
    ]
    .map(|path| doc.show_file(path))
    .join("");
    assert!(twin_source.contains("BUILDER faithful-build"));
    assert_eq!(twin_source.matches("FROM alpine:3.22").count(), 1);
    assert!(twin_source.contains("APP dissolved\nIMPORT ${pkgs.hello}\nSTART hello"));
    let faithful = doc.sh(
        "faithful_item=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.twin-workspaces cix build twins | jq -r '.faithful'); printf '%s\\n' \"$faithful_item\"",
        true,
    );
    let faithful_path = faithful
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("faithful build printed its captured item")
        .to_owned();
    let dissolved = doc.sh(
        "dissolved_item=$(cix build --file Cixfile.dissolved twins | jq -r '.dissolved'); printf '%s\\n' \"$dissolved_item\"",
        true,
    );
    let dissolved_path = dissolved
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("dissolved build printed its captured item")
        .to_owned();
    let faithful_run = doc.sh_with_env(
        "cix run \"$faithful_item\" --user",
        &[("faithful_item", &faithful_path)],
        true,
    );
    assert!(faithful_run.contains("Hello, world!"));
    stop_empty_cix_run_slice("the faithful translation receipt");
    let dissolved_run = doc.sh_with_env(
        "cix run \"$dissolved_item\" --user",
        &[("dissolved_item", &dissolved_path)],
        true,
    );
    assert!(dissolved_run.contains("Hello, world!"));
    stop_empty_cix_run_slice("the dissolved translation receipt");

    doc.para("The directory argument is the build context. `--file Cixfile.dissolved` is resolved inside `twins`, not in the shell's current directory, so the two commands above select `twins/Cixfile` and `twins/Cixfile.dissolved` respectively.");
    let locks = doc.sh("ls -1 twins/Cixfile*.lock", true);
    assert!(locks.contains("twins/Cixfile.lock"));
    assert!(locks.contains("twins/Cixfile.dissolved.lock"));
    let revisions = doc.sh(
        "for lock in twins/Cixfile*.lock; do printf '%s: ' \"$lock\"; jq -r '.inputs.pkgs.rev' \"$lock\"; done",
        true,
    );
    assert_eq!(
        revisions
            .matches("624af665418d3c65d544145b4d34ad696439570e")
            .count(),
        2
    );
    doc.para("Both alternatives still need independent locks even without FETCH: each moving `FROM ... nixos-unstable` binder is pinned to the immutable revision printed above. Commit both sibling lock files. `cix build twins --update-lock` rewrites only `twins/Cixfile.lock`; `cix build twins --file Cixfile.dissolved --update-lock` rewrites only `twins/Cixfile.dissolved.lock`, so testing an update cannot silently change the other translation.");

    doc.para("## Continue with real migrations");
    doc.para("For your own project, first list the source Dockerfile's produced files, entrypoint arguments, ports, writable paths, and external inputs. Write a faithful Cixfile that reproduces those observable facts, build and probe it, then ask the decision rule above package by package. Replace ceremony with direct imports only when the receipts remain equal, keep every network input pinned, and commit the selected Cixfile with its lock.");
    doc.para("[docs/migrate.md](../migrate.html) contains the directive-by-directive mapping, including multi-stage builds, users, volumes, entrypoints, and the places where no mechanical translation is safe. The [migration corpus browser](../corpus/index.html) contains worked real-project pairs. There, **source context** means the original project files and revision, a **receipt** is the command and observed result used to grade a translation, and a **gap** is an explicit behavior the current Cixfile or cix implementation does not yet reproduce. In this chapter the five-line Dockerfile is the source context, the two `Hello, world!` runs are receipts, and there is no remaining behavior gap for this tiny program.");
    doc.finish()
}
