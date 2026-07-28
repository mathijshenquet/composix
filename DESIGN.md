# composix — design

A toolkit on nix + systemd meant to replace docker. CLI: `cix`.

Docker analogy map: registry/tag ↔ **index** (part 1) · image metadata ↔ **spec** (part 2) ·
compose.yml ↔ **compose** (part 3) · Dockerfile ↔ **Cixfile** (part 4).

Status legend: ✅ decided · 🔶 position taken, review welcome · ❓ open, needs discussion.

## Decisions so far

- ✅ D1: Implementation language: Rust. Single `cix` binary (workspace may split crates).
- ✅ D2: Runtime target: **system systemd, root-managed**. Units live in `cix-<composite>.slice`,
  isolation via `DynamicUser=` + systemd sandboxing. `cix run` uses transient units (`systemd-run`).
- ✅ D3: Build order: **index** (part 1) and **spec + run** (part 2) developed in parallel by agents;
  compose (3) and Cixfile (4) after, compose only after serious surface-language prototyping.
- ✅ D4: Cixfile is Dockerfile-ish syntax; writing a plain `.nix` instead is always a first-class
  escape hatch.
- ✅ D5: Tags name **built outputs** (store paths / closures), not derivations. `.drv` path may be
  recorded as provenance metadata. "Build on pull" is out of scope.
- ✅ D6: Index does **not** host store contents. An index entry points at substituters
  (attic, nix-serve, S3, harmonia, …) where the closure can be fetched.
- ✅ D7: A local tag is a **nix GC root**. Tagging pins, untagging unpins.
- ✅ D8: The spec is a file **in the output**: `$out/cix-spec.json` — inspectable without nix
  (lingua franca). One store item can declare multiple services.
- ✅ D9: Compose mechanism is **resolve → lock → build → activate**: tags are mutable, nix is not;
  the bridge is a resolve step writing `cix.lock`, then a per-composite derivation of generated
  units, activated via a per-composite nix profile (atomic upgrade + rollback per composite).
- ✅ D10 (was O2): `cix serve --with-store` is in the MVP: `nix copy --to file://<dir>` materializes
  a standard binary cache (narinfo/nar, optional `nix store sign`), served statically by the same
  process; the serve URL is then advertised in entries' `substituters`. Zero-infra sharing; the
  index remains the value, the store serving is ~free (D6 otherwise unchanged).
- ✅ D11 (was O3): dirs use the **app-path model** (docker `VOLUME`-like): the spec declares the
  absolute paths the app touches; the generator realizes them via `*Directory=` on the host plus
  `BindPaths=` into the service's mount namespace. Uniform for stubborn and well-behaved apps alike
  (for the latter, the spec simply picks the canonical path it passes via flags/env).
- ✅ D13: `cix run --user` is an **explicitly degraded dev mode** against the user manager (no
  `DynamicUser`, sandboxing subset, loudly labeled in output). Exists for dev loops and root-free
  testing; the system manager remains the product target (D2 unchanged).
- ✅ D14: index entries are **per-system**: `"outputs": {"x86_64-linux": {storePath, narHash}, …}`.
  `cix tag` fills the slot for the tagged path's system; tagging the same ref from another builder
  adds a slot; `cix pull` selects the current system (error if absent).
- ✅ D15: spec parsing **rejects unknown fields** (no silent ignore) — keeps specs honest and makes
  `cixSpec` version bumps meaningful.
- ✅ D16 — positioning: **baseline composix (parts 1–3) is a complete product for the nix-native
  audience** (they have store items; they lack naming/distribution and service-ization). The
  Cixfile is the adoption bridge for non-nix users, not the foundation. Intermediate rung, cheap
  and early: a nix lib helper (`composix.lib.withSpec drv {…}`) attaching a spec to any existing
  derivation.

---

## Part 1 — index

### Concepts

- **ref**: `[{root_url}/]{name}:{tag}`. `name`, `tag`: `[a-z0-9._-]+`; `name` may contain `/`
  (path segments; the first segment must not look like a host). Tag defaults to `latest`.
  **root_url is `host[:port]` only — no path prefixes** (refined D12): in
  `cix.example.com/team/my-app:v3` the origin is `cix.example.com` and the *name* is
  `team/my-app` — team/project namespacing lives inside the name, exactly like docker registries.
  The HTTP API anchors at a fixed root (`https://{host}/v1/…`), which is what keeps the grammar
  unambiguous; path-prefix hosting can be added later via `/.well-known` discovery without
  touching the ref grammar.
  ```
  ref        = [host "/"] name [":" tag]        (tag defaults to latest)
  host       = contains "." or ":port", or "localhost"
  local  ref = no host  → never resolved remotely, never served
  remote ref = host     → API at https://host/v1/, self-describing anywhere
  ```
  No default registry, ever (docker.io's magic is refused): bare names are genuinely local, so
  nothing bare touches the network or gets served — this kills podman's short-name
  ambiguity/squatting class outright. Digest pinning needs no `@digest` syntax: the store path
  *is* the digest and is accepted anywhere an installable is; `cix.lock` pins store paths.
- ✅ **D12 (was O1) — ref model**: docker-style self-describing refs, not git-style named
  remotes. Three separations that make it clean:
  1. *The ref is an identity, not an address.* Disambiguation rule (docker's): first
     slash-component containing `.` or `:port`, or equal to `localhost` ⇒ root_url; otherwise the
     whole ref is a local name (which may contain `/`).
  2. *Serving claims an identity, not a socket.* `cix serve <root_url> --listen <addr>` serves
     exactly the tags prefixed `<root_url>`; DNS/reverse proxy maps identity → socket. In dev,
     `localhost:8420` is itself a valid root_url.
  3. *Publishing is tagging into the namespace.* `cix tag my-app:v3 cix.example.com/my-app:v3`
     adds a second tag to the same store path; `cix ls` shows what is public under which identity;
     nothing is served by accident.
  Git-style upstream tracking survives underneath (pulled/aliased tags record their origin);
  named remotes as a *surface* were rejected: refs stop being self-describing, which is the
  "publicize our-app:v321" use case itself.
- **entry** (what a ref resolves to; per-system per D14):
  ```json
  {
    "outputs": {
      "x86_64-linux":  { "storePath": "/nix/store/…-my-app", "narHash": "sha256-…",
                         "drvPath": "/nix/store/…-my-app.drv" },
      "aarch64-linux": { "storePath": "…", "narHash": "…" }
    },
    "substituters": ["https://cache.example.com"],
    "trustedKeys": ["cache.example.com-1:…"],
    "createdAt": "…"
  }
  ```
  (`drvPath` optional, provenance only. `cix pull` selects the current system, errors if absent.)
- **local index**: per-user, at `~/.local/state/cix/`. Layout: a symlink farm
  `roots/{name}:{tag} → /nix/store/…` (each registered as an indirect nix GC root, so the DB is
  `ls`-able and doubles as the pin) plus a JSON sidecar per tag for metadata (upstream, narHash,
  timestamps). No sqlite in v0.
- **upstream**: a tag pulled from a remote records its origin `root_url` (git-remote-like), used by
  bare `cix pull` to refresh.

### CLI

- `cix tag <installable> <ref>` — installable = store path, flake installable (`.#foo`), or an
  existing ref. Resolves/builds → symlink + gcroot + sidecar. Tagging with a `root_url` prefix marks
  the tag as publishable under that origin.
- `cix untag <ref>`; `cix ls [prefix]` — list tags, `-l` shows store path / upstream / age.
- `cix serve <root_url> [--listen host:port] [--substituter url]... [--with-store [--sign-key file]]`
  — HTTP resolver serving exactly the tags prefixed with `root_url`. Advertised substituters come
  from serve config; `--with-store` additionally maintains + statically serves a
  `nix copy --to file://` binary cache and advertises itself as a substituter (D10).
- `cix pull <root_url>/<name>:<tag> [--as <name[:tag]>]` — resolve over HTTPS, `nix copy` the
  closure from advertised substituters (verify narHash; require signatures when trustedKeys given),
  then tag locally with upstream recorded.
- `cix pull` — refresh every tag that has an upstream; fetch the ones that moved.

### HTTP API (v1, JSON)

- `GET {root_url}/v1/resolve/{name}/{tag}` → entry (404 if unknown)
- `GET {root_url}/v1/tags/{name}` → `{"tags": {"latest": <entry>, …}}`
- `GET {root_url}/v1/names` → `{"names": ["my-app", …]}`

Trust model: integrity/authenticity ride on nix path signing + narHash verification, **not** on
trusting the index. The index is a resolver; TLS protects the pointer, nix signatures protect the
content.

### Open

- ❓ O1: serve-only federation agreed in principle; the docker-style ref model above awaits
  Mathijs's confirmation (alternative considered and disfavored: git-style named remotes).
  `cix push` + authenticated central index: later.
- ~~O2~~ → resolved as D10 (`--with-store` in MVP).

## Part 2 — spec + run

Key alignment: systemd already has the spec's primitives first-class (`StateDirectory=`,
`CacheDirectory=`, `LogsDirectory=`, `ConfigurationDirectory=`, `DynamicUser=`, `LoadCredential=`,
sandboxing suite). The spec is a runtime-neutral capability declaration that compiles down to
those. Everything **not** declared is denied.

### Schema v0 — `$out/cix-spec.json`

```json
{
  "cixSpec": 1,
  "services": {
    "my-app": {
      "exec": ["bin/my-app", "--port", "$PORT"],
      "env": {
        "PORT":   { "type": "port", "default": 8000 },
        "DB_URL": { "type": "string", "required": true, "secret": true }
      },
      "ports": { "http": { "env": "PORT", "protocol": "tcp" } },
      "dirs": {
        "state": ["/var/lib/my-app"],
        "cache": ["/var/cache/my-app"],
        "logs":  ["/var/log/my-app"]
      },
      "health": { "exec": ["bin/healthcheck"], "interval": "30s" }
    }
  }
}
```

- `exec`: argv; relative paths resolve against `$out`; `$VAR` interpolation from declared env only.
- `env`: typed (`string|int|bool|port|path`), `default`/`required`/`secret`. `secret: true` is
  schema-reserved in v0 (enforced delivery via `LoadCredential=` comes with compose).
- `ports`: named; each binds to an env var carrying the actual port number. Declaring any port ⇒
  network access granted; no ports and no `network: "host"` ⇒ `PrivateNetwork=yes`.
- `dirs`: writable dirs by role, as the **absolute paths the app touches** (D11, docker
  `VOLUME`-like). Realized as `*Directory=` on the host + `BindPaths=` into the unit's mount ns;
  everything else in the filesystem stays read-only/hidden. Well-behaved apps simply have their
  canonical path passed via `exec` args — one uniform model.
- `health`: optional; v0 may parse-and-ignore, wired to watchdog/readiness later.

### Unit generation (spec → systemd)

| spec | unit |
| --- | --- |
| exec | `ExecStart=` (absolute store paths) |
| env defaults / `-e` overrides | `Environment=` |
| dirs.state/cache/logs/config | `StateDirectory=` / `CacheDirectory=` / `LogsDirectory=` / `ConfigurationDirectory=` |
| ports declared | `RestrictAddressFamilies=+AF_INET +AF_INET6` |
| no network | `PrivateNetwork=yes` |
| always | `DynamicUser=yes`, `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`, `NoNewPrivileges=yes`, `RestrictSUIDSGID=yes`, `ProtectKernelTunables/Modules/Logs=yes`, `ProtectControlGroups=yes`, `LockPersonality=yes`, `MemoryDenyWriteExecute=yes` (opt-out for JITs — needs a spec flag), `SystemCallFilter=@system-service`, `CapabilityBoundingSet=` |

Strict-by-default is the point: the spec *is* the capability grant.

### `cix run`

- `cix run <installable-or-ref>[#service] [-e K=V]... [-p name=port]... [--detach] [--user]`
  (`--user` = degraded dev mode per D13)
- One service in spec ⇒ `#service` optional. Validates env against schema (types, required).
- Transient unit via the system manager (`systemd-run` semantics over D-Bus):
  `cix-run-<service>-<nonce>.service` in `cix-run.slice`. Foreground: stream journal, ctrl-C stops
  unit. `--detach` prints unit name; `cix ps` lists `cix-*` units.

### Open

- ~~O3~~ → resolved as D11 (app-path model).

## Part 3 — compose (design pending, mechanism fixed)

Mechanism per D9. Not yet designed in detail; parked notes:

- Surface language: **undecided — requires serious prototyping** (candidates: TOML, a
  Cixfile-sibling DSL, restricted nix; evaluate by writing the same non-trivial composite in each).
- Unit naming: `cix-<composite>-<service>[@<n>].service`, `cix-<composite>.slice`,
  `cix-<composite>.target` → "what runs here" = `systemctl list-units 'cix-*'`.
- `cix.lock` pins ref → entry (store path + narHash), exactly flake.lock's role.
- Reconciler: `cix up` is one-shot; a daemon/timer re-resolving moved tags is the k8s-lite seed,
  cleanly separable. Update policy (restart order, hold/watch tags, later blue-green) is
  per-service compose config.
- Networking: MVP is host networking with explicit port allocation in compose. Per-slice netns
  later, consciously deferred.
- Secrets: compose-level concern → `LoadCredential=`; spec only marks `secret: true`.

## Part 4 — Cixfile (design pending)

- Positioning per D16: not essential to baseline value, but where the non-nix audience is won.
  Sequenced last; `composix.lib.withSpec` (nix helper) lands much earlier as the nix-native rung.
- Two halves: BUILD (project → store item) and SERVICE (→ spec). SERVICE half is settled by the
  part-2 schema; blocks compile to `cix-spec.json`.
- BUILD half MVP position 🔶: no general imperative RUN steps (impurity). Recognize ecosystems
  (cargo/pnpm/uv/go + their lockfiles → nixpkgs deterministic builders); plus the D4 escape hatch
  (write nix yourself). Discipline: Cixfile stays sugar over a small set of blessed builders —
  never a general build system. General build steps: later, if ever.

## Non-goals (for now)

Hosting nars (D6, modulo O2) · multi-host orchestration · per-service netns · build-on-pull ·
non-systemd runtimes.
