use super::super::*;

pub(crate) fn chapter_cixfile_language() -> String {
    let mut doc = Doc::new("cixfile-language");
    fs::write(doc.base.join("index.html"), "guide site\n").expect("writing language fixture page");
    fs::write(
        doc.base.join("service.conf"),
        "root=/srv/guide-site\nstate=/var/lib/guide-site\n",
    )
    .expect("writing language fixture config");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE guide-site
IMPORT ${pkgs.coreutils} ${pkgs.busybox} ${pkgs.bash}
COPY index.html /srv/guide-site/index.html
COPY ${pkgs.coreutils}/bin/printf /opt/tools/printf
COPY ${pkgs.nginx}/conf /opt/nginx
COPY service.conf /etc/guide-site/service.conf
FILE /etc/guide-site/build-origin <<ORIGIN
packages=${pkgs.coreutils}
ORIGIN
START sleep 60
ENV SITE_NAME=guide
ENV API_TOKEN required
STATEDIR /var/lib/guide-site
STATEDIR /opt/nginx/state
CACHEDIR /var/cache/guide-site
LOGDIR /var/log/guide-site
CONFIGDIR /etc/guide-site
RUNDIR /run/guide-site
PORT web = 8088
PORT dns = udp:5353
LISTENER admin
CLAIM egress
CLAIM jit
"#,
    )
    .expect("writing language Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing language lock");

    doc.para("You will expand the first service into an example of the everyday Cixfile declarations. Each declaration either names an input, assembles the item's filesystem, or grants a narrowly described runtime capability.");

    doc.para("## A graph you can read from top to bottom");
    doc.para("A **binder** is a name introduced by `AS`, `FETCH`, or `BUILDER` and referenced later as `${name}`. A SERVICE, APP, or ITEM block produces an **artifact**—the final immutable store item, as distinct from a temporary builder workspace. References point only backward, so the file is a graph that can be understood from top to bottom without an implicit starting filesystem.");
    doc.para("Here is the rule in five lines on each side. The Dockerfile column repeatedly changes one implicit build filesystem; the Cixfile column names the temporary `make` tree and then copies one explicit result into `output`.");
    doc.para("| Dockerfile (five lines) | Cixfile (five lines) |\n| --- | --- |\n| `FROM alpine:3.22` | `BUILDER make` |\n| `WORKDIR /work` | `COPY message .` |\n| `COPY message .` | `RUN tr a-z A-Z < message > result` |\n| `RUN tr a-z A-Z < message > result` | `ITEM output` |\n| `RUN chmod 0444 result` | `COPY ${make}/result /result` |");
    doc.para("In the full example, `FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs` is resolved to the immutable revision recorded in `Cixfile.lock`; it supplies packages, not a mutable base filesystem. `FROM . AS src` names this Cixfile's directory, and `${src}/index.html` therefore means that checked-in file. Bare `COPY index.html …` is the deliberate shorthand for the same local source.");
    let source = ["Cixfile", "index.html", "service.conf"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("FROM . AS src"));
    assert!(source.contains("SERVICE guide-site"));
    assert!(source.contains("COPY index.html /srv/guide-site/index.html"));
    assert!(source.contains("ENV API_TOKEN required"));
    assert!(source.contains("PORT dns = udp:5353"));
    assert!(source.contains("CLAIM egress"));

    doc.para("## IMPORT and the store-aware copy rule (CIP-91)");
    doc.para("`IMPORT` unions each package's `bin`, `etc`, and `share` trees at those same destinations in the item; paths outside those trees are not imported. Earlier imports win a collision, so coreutils supplies `ls` even though busybox follows it.");
    doc.para("The **provenance** of a COPY source is simply its declared origin: local context, package, FETCH, builder, or another item. Local bytes are **materialized**, meaning an ordinary real copy like Docker's `COPY`. Store-backed sources normally become symbolic links whose targets are immutable `/nix/store` paths; the item's Nix closure records those targets so copying the closure to another machine brings every runtime dependency too.");
    doc.para("A writable runtime mount cannot be placed below a symlinked directory: the mount namespace needs a real ancestor directory. The store-aware copy rule (CIP-91) therefore copies the exact `/opt/nginx` subtree because `STATEDIR /opt/nginx/state` sits below it, while the unrelated `printf` file remains a link.");
    let built = doc.sh(
        "item=$(cix build .#guide-site); printf '%s\\n' \"$item\"",
        true,
    );
    let store_path = built
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected guide-site build printed an item")
        .to_owned();
    let linked = doc.sh_with_env(
        "ls -l \"$item/opt/tools/printf\"",
        &[("item", &store_path)],
        true,
    );
    assert!(linked.contains(" -> /nix/store/"), "{linked}");
    let materialized = doc.sh_with_env(
        "test ! -L \"$item/opt/nginx\" && printf 'opt/nginx is a materialized directory\\n'",
        &[("item", &store_path)],
        true,
    );
    assert_eq!(materialized.trim(), "opt/nginx is a materialized directory");

    doc.para("`FILE` creates the small interpolated `build-origin` file below. It is useful when the content genuinely needs a binder value; for ordinary configuration it is a smell, because a checked-in file plus `COPY` stays easier to lint, edit, and test.");
    let generated = doc.show_file(Path::new(&store_path).join("etc/guide-site/build-origin"));
    assert!(generated.contains("packages=/nix/store/") && generated.contains("-coreutils-"));

    doc.para("## Runtime declarations are grants");
    doc.para("`ENV SITE_NAME=guide` supplies a default. `ENV API_TOKEN required` names a required non-secret operator value; bare `ENV THEME` declares an optional value that stays unset unless supplied. Direct run supplies a value as `cix run \"$item\" -e API_TOKEN=example`, while a compose child uses `\"env\": {\"API_TOKEN\": \"example\"}`. Secret values instead use `SECRET` and the credential-file mechanism described below.");
    doc.para("Role directories use the application's native absolute paths. Systemd creates unit-scoped backing below the host's state, cache, log, configuration, and runtime roots and binds it to the declared path: state survives until explicit purge, cache is expendable, logs are retained until cleaning policy removes them, writable config is operator-managed, and run data disappears on stop. An operator can replace a declared role with existing content using `--dir /etc/guide-site=host:/srv/guide-config --identity guide-site`; compose places the same `host:` materialization in the child's `dirs` map. For a compose named `stack`, `cix clean stack --what cache` removes only expendable cache, while `cix down stack --purge --yes` explicitly removes cix-owned state and shared data; host-backed `DIR` data is never deleted.");
    doc.para("A bare port is TCP; the `udp:` prefix is the single UDP spelling. `LISTENER admin` declares no address: the operator assigns one with `-p admin=127.0.0.1:8420`, systemd owns that TCP socket, and the process receives file descriptor 3 with `LISTEN_FDS=1` and `LISTEN_FDNAMES=admin`. Compose publishes a named listener in Chapter 6.");
    doc.para("Claims form a closed vocabulary: `egress` permits outbound networking, `jit` drops `MemoryDenyWriteExecute=`, `gpu` opens the `/dev/dri` class, and `device /dev/name` opens exactly one device. Without egress the compiler uses a private or deny-by-default network; without jit writable executable memory stays denied. These declarations still describe the intended unit under `--user`, but an incapable user manager may emit the degradation marker taught in Chapter 1.");
    let manifest = doc.sh_with_env(
        "jq '{env, ports, listeners, dirs, claims}' \"$item/cix-manifest.json\"",
        &[("item", &store_path)],
        true,
    );
    assert!(manifest.contains("\"udp\""));
    assert!(manifest.contains("\"admin\""));
    assert!(manifest.contains("\"required\": true"));
    assert!(manifest.contains("\"egress\""));
    assert!(manifest.contains("\"jit\""));

    doc.para("## The remaining runtime grammar");
    doc.para("`START` is the main argv. `START_PRE` is run before every initial start and restart, so it must be safe to repeat after a partial attempt. `SERVICE` stays running; `APP` is a systemd oneshot whose exit status is the result; `ITEM` is only a store tree with no manifest, so it can be copied from or tagged but not run.");
    doc.para("`SECRET db-password AS DB_PASSWORD_FILE` declares a credential need without a value. Compose supplies `\"secrets\": {\"db-password\": {\"file\": \"/etc/cix/db-password\"}}`; systemd mounts the root-owned source at `$CREDENTIALS_DIRECTORY/db-password` and sets `DB_PASSWORD_FILE` to that path. `DIR /media:ro` instead declares pre-existing operator data: cix neither creates nor deletes it, and the operator maps it with a `host:`, `shared:`, or role alias materialization.");
    doc.para("Health declarations use URL targets: `READINESS http://127.0.0.1:8080/healthz IN 30s` waits up to 30 seconds for the first successful HTTP response before startup succeeds, while `LIVENESS tcp://127.0.0.1:8080 EVERY 10s` probes repeatedly and gives systemd a three-interval watchdog window before restart. With exactly one declared PORT, `/healthz` is sugar for its localhost HTTP URL. `notify` stays bare when the program speaks systemd notify itself. `SHM 64M` creates a private `/dev/shm` tmpfs with that size limit.");

    doc.para("## Directive reference");
    doc.para("| Declaration | What it adds |\n| --- | --- |\n| `FROM … AS name` | A package/source/item binder pinned in `Cixfile.lock`; `FROM .` names unpinned local context. |\n| `FETCH name command … EXPECT hash` | The only networked step; it binds pinned downloaded output. |\n| `BUILDER name` | A reusable workspace under `~/.cache/cix/workspaces` by default; delete that cache to reclaim it without changing correctness. |\n| `SERVICE` / `APP` / `ITEM` | A long-running unit / finite oneshot / non-runnable store tree. |\n| `IMPORT package…` | An earlier-wins read-only package union with bare command lookup. |\n| `COPY source /destination` | Store-aware item assembly; builder destinations are workspace-relative. |\n| `FILE /destination <<EOF` | An inline interpolated file; prefer checked-in files when possible. |\n| `START` / `START_PRE` | Main argv / repeat-safe service pre-start argv. |\n| `ENV` / `SECRET` | Non-secret runtime configuration / compose-supplied credential file. |\n| `PORT` / `LISTENER` | A direct TCP/UDP bind / systemd-owned TCP socket. |\n| role dirs / `DIR` | Cix-owned lifecycle storage / operator-owned data. |\n| `READINESS` / `LIVENESS` | Startup gate / watchdog restart probe. |\n| `CLAIM` / `SHM` | A named sandbox exception / size-bounded private tmpfs. |");
    doc.finish()
}
