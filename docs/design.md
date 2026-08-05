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
- ✅ D8: The manifest is a file **in the output**: `$out/cix-manifest.json` — inspectable without nix
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
  `cixManifest` version bumps meaningful.
- ✅ D16 — positioning: **baseline composix (parts 1–3) is a complete product for the nix-native
  audience** (they have store items; they lack naming/distribution and service-ization). The
  Cixfile is the adoption bridge for non-nix users, not the foundation. Intermediate rung, cheap
  and early: a nix lib helper (`composix.lib.withSpec drv {…}`) attaching a spec to any existing
  derivation.
- ✅ D17 (v2; supersedes the claims design) — **serve exposes the bare tag DB; qualification is
  reachability.** `cix serve` takes no root_url: it serves all *bare* local tags under whatever
  URL reaches it. A fully-qualified name is nothing but the URL of a bare tag on the store that
  DNS resolves the host to. Consequences: `cix tag` into a qualified name is a hard error
  (qualified names denote remote state, exclusively); pull-created mirror tags are never
  re-served — to re-serve, adopt as bare (`cix pull … --as`), which is the entire adoption
  story. `cix publish` is deliberately NOT built now; later it means *ask a server to publish
  for you* (push-shaped, with server-side authorization — where the claims idea returns as
  ACLs).
- ✅ D18 (v2) — **the ref is a URL; one negotiated URL space** (in the spirit of the web):
  `GET https://host/{name}[:{tag}]` serves an informative HTML page to browsers and the entry
  JSON to `Accept: application/vnd.cix+json;version=1` (the header the cix client always
  sends); `GET /` lists names. API versioning lives in the media type, not the path — there is
  no `/v1/`. `Vary: Accept`; `?format=json` as the human escape hatch; `/store/` (nix
  binary-cache protocol) unchanged. Prior art: ActivityPub/Mastodon conneg, Linked-Data "Cool
  URIs", GitHub-style vnd media types. Pages self-reference via the request's Host header.
- ✅ D19 — **literate documentation** (gitsitter `tests/workflows.rs` pattern): docs are generated
  from executed scenarios driving the real `cix` binary in isolated environments; assertions make
  them tests, transcripts make them docs, and a drift-check test keeps the committed markdown
  current. Outputs are normalized (store hashes, timestamps, ages) so generation is deterministic.

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
  ambiguity/squatting class outright. **Remotes are never *named*** (no git-style `origin`
  indirection): the fully-qualified name — a URL — *is* the identity. Digest pinning needs no
  `@digest` syntax: the store path *is* the digest and is accepted anywhere an installable is;
  `cix.lock` pins store paths.
- ✅ **D12 (was O1) — ref model**: docker-style self-describing refs, not git-style named
  remotes. Three separations that make it clean:
  1. *The ref is an identity, not an address.* Disambiguation rule (docker's): first
     slash-component containing `.` or `:port`, or equal to `localhost` ⇒ root_url; otherwise the
     whole ref is a local name (which may contain `/`).
  2. *Serving exposes a store, not an identity.* `cix serve` publishes the local **bare** tag DB
     at whatever URLs reach its socket — the server does not know or configure its own name;
     which hostnames route to it is DNS's business (exactly like a web server and its vhosts).
  3. *Publishing is tagging on a served store.* A bare tag on a box that serves is public under
     that box's URLs; nothing else is. `my-org` publishing = `ssh index-box cix tag …` (or CI).
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
  existing ref. Resolves/builds → symlink + gcroot + sidecar. The target ref must be **bare**;
  a qualified target is a hard error (D17: qualified names denote remote state; to publish,
  tag on the box that serves).
- `cix untag <ref>`; `cix ls [prefix]` — list tags, `-l` shows store path / upstream / age.
- `cix serve [--listen host:port] [--substituter url]... [--with-store [--sign-key file]]`
  — serves the **bare** tag DB (D17) at whatever URL reaches it. Advertised substituters come
  from serve config; `--with-store` additionally maintains + statically serves a
  `nix copy --to file://` binary cache and advertises itself as a substituter (D10).
- `cix pull <root_url>/<name>:<tag> [--as <name[:tag]>]` — resolve over HTTPS, `nix copy` the
  closure from advertised substituters (verify narHash; require signatures when trustedKeys given),
  then tag locally with upstream recorded.
- `cix pull` — refresh every tag that has an upstream; fetch the ones that moved.
- `cix inspect <ref-or-installable>` — a bare ref inspects local state; a qualified ref uses the
  same Docker-style grammar as `pull`, negotiates the index entry at that URL, and prints the
  normal pretty JSON without creating a local mirror tag.

### The org workflow (pre-push)

Publishing to `cix.my-org.com` = getting a bare tag onto the box DNS resolves it to — the
git-before-forges model (you have write access, so "push" is ssh):

```
nix copy --to ssh://index-box /nix/store/…-myapp
ssh index-box cix tag /nix/store/…-myapp myapp:v1
# a running `cix serve --with-store` picks it up; org-wide:
cix pull cix.my-org.com/myapp:v1
```

Later, `cix publish`/`cix push` abstracts exactly those two lines (ssh transport first,
authenticated HTTP for docker.io-style registries after), and authorization enters server-side.

### HTTP surface (one negotiated URL space, D18)

The URL space IS the name space; representation is negotiated (`Vary: Accept`,
`?format=json|html` escape hatch):

| URL | browser (HTML) | `Accept: application/vnd.cix+json;version=1` |
| --- | --- | --- |
| `/` | list of served names, linked | `{"names": [...]}` |
| `/{name}` | tag table (tag, systems, closure size, narHash, age), pull snippet, spec summary if the store path carries `cix-manifest.json` | `{"tags": {"latest": <entry>, …}}` |
| `/{name}:{tag}` | that entry's detail, provenance, `cix pull`/`cix run` snippets — the permalink you publicize | `<entry>` (404 if unknown) |
| `/store/…` | — | nix binary-cache protocol (D10), no negotiation |

HTML pages construct their self-referential names/snippets from the request's Host header.

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

### Schema v0 — `$out/cix-manifest.json`

```json
{
  "cixManifest": 1,
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
| always | `DynamicUser=yes`, `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`, `PrivatePIDs=yes`, `NoNewPrivileges=yes`, `RestrictSUIDSGID=yes`, `ProtectKernelTunables/Modules/Logs=yes`, `ProtectControlGroups=yes`, `LockPersonality=yes`, `MemoryDenyWriteExecute=yes` (opt-out for JITs — needs a spec flag), `SystemCallFilter=@system-service`, `CapabilityBoundingSet=` |

Strict-by-default is the point: the spec *is* the capability grant.

`PrivatePIDs=yes` makes the service entrypoint namespace PID 1. The application is therefore
responsible for PID-1 duties: it must reap children and handle signals explicitly. Master/worker
daemons such as nginx and PostgreSQL already do this; an item that does not is evidence for a
future declarative init-shim grant, not a reason to weaken the default.

### `cix run`

- `cix run <installable-or-ref>[#service] [-e K=V]... [-p name=port]... [--detach] [--user]`
  (`--user` = degraded dev mode per D13)
- One service in spec ⇒ `#service` optional. Validates env against schema (types, required).
- Transient unit via the system manager (`systemd-run` semantics over D-Bus):
  `cix-run-<service>-<nonce>.service` in `cix-run.slice`. Foreground: stream journal, ctrl-C stops
  unit. `--detach` prints unit name; `cix ps` lists `cix-*` units.

### Spec v2 (decided 2026-07-28, after the nginx/postgres dogfood round)

Two principles first, more important than any field:

- **D20a — no raw systemd passthrough, ever.** The spec declares capabilities in *app
  semantics* ("I JIT", "I listen on 80", "I need a socket dir"); the generator maps to
  mechanism. Raw overrides are operator territory → compose, never the spec.
- **D20b — the boundary**: spec = what the supervisor must know (lifecycle, resources,
  capabilities); item = app quirks (nss_wrapper, path plumbing); compose = operator decisions
  (overrides, config content, scaling).

v2 fields/changes (`"cixManifest": 2`; runner accepts 1 and 2; new fields rejected under 1):

1. `dirs.run` — runtime-lifetime role → `RuntimeDirectory=` (tmpfs, wiped on stop): sockets,
   pidfiles.
2. `setup: [argv]` — pre-start hook, ExecStartPre semantics: runs *every* start in the same
   sandbox, MUST be idempotent (the docker-entrypoint / k8s-initContainer convergence;
   "first run" is undefinable — the state itself is the only truth).
3. Fixed-value ports: `{"value": 8080}` alongside the env form, for env-blind apps (nginx);
   `-p` override of a value port is a clear error.
4. Declared port < 1024 ⇒ generator grants exactly `CAP_NET_BIND_SERVICE` (Ambient +
   BoundingSet). No new field: the declaration is the grant. Under CIP-84 closed roots,
   `PrivateUsers=` makes that capability ineffective against the host network namespace, so
   compilation instead rejects the direct bind and teaches an unprivileged port or named
   `LISTENER`; systemd socket activation owns privileged binds.
5. `jit: true` ⇒ drop `MemoryDenyWriteExecute`. Semantic name, per D20a.
6. **D11 narrowed**: a role's app path MUST live under that role's conventional root
   (state→`/var/lib`, cache→`/var/cache`, logs→`/var/log`, config→`/etc`, run→`/run`), one
   component deep. Reason: systemd's idmapped managed dirs can only be aliased within their
   root (`StateDirectory=cix-…:name`); a `BindPaths` to an arbitrary path provably loses the
   ID map on systemd 257. The restriction codifies what the platform can actually deliver —
   and matches FHS anyway.

Deferred consciously: health/readiness wiring (compose era), nss_wrapper standardization
(app quirk; reconsider if it recurs across cixpkgs), devices/GPU (no dogfood case yet),
served-payload dirs à la `/srv` (immutable payload lives in the store; mutable shared payload
is a compose question).

Amendments (same day, after the YAGNI round with Mathijs):

- **D21 — env de-typing.** The `type` field is dropped from the schema docs: env entries are
  `{default?, required?, secret?}` (all values strings). Port-ness is *structural*: a var is a
  port because the `ports` section references it; cix validates the referenced var's
  default/override parses as a port, and the <1024 capability logic keys off that. `int`,
  `bool`, `path` bought a marginally earlier error at real complexity cost — removed. For
  compat, `type` is still *accepted and ignored* (deprecated) within v2; hard removal at v3.
- **D22 (v3, supersedes the /item and /app mounts) — filesystem projection.** The item is a
  **sparse rootfs fragment**: it mirrors absolute paths (`$out/etc/nginx/nginx.conf`,
  `$out/srv/www/…`), and the spec carries an explicit `"mounts": ["/etc/nginx", …]` list
  (generated by `cix build` from declared destinations; hand-writable in `.nix` items). The
  runtime projects each mount as a read-only bind into the unit's namespace — empirically
  verified 2026-07-28: systemd creates mountpoints for nonexistent destinations (any depth,
  incl. root-level) during namespace assembly under full hardening. Two zones: destinations
  starting with `/` are projected; bare relative paths (`bin/…`) are item-internal (exec/setup
  targets — the ExecStart-resolves-pre-namespace constraint stands, so exec argv stays
  store-path-based). Validation: a mount may not overlap a role-dir path
  (immutable-in-item XOR operator-writable, never both on one path — forced clarity), and a
  small deny-list (`/nix`, `/proc`, `/sys`, `/dev`, role roots, identity-critical files like
  `/etc/passwd`). No `/app`, no stable-mount env var: configs live at their native
  conventional paths and file contents stay verbatim. Degraded `--user` mode cannot project;
  it warns loudly and exposes the item's store path via `CIX_APP`.

### Open

- ~~O3~~ → resolved as D11 (app-path model), narrowed by Spec v2 point 6.

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

## Part 4 — Cixfile v1 (assembly subset — decided 2026-07-28)

Scope: *assembly*, not building — wrap nixpkgs packages + files + a spec into a runnable
item. Ecosystem builds (cargo/pnpm/uv) stay out of v1 entirely.

Item directives: `PKG <attr>` (nixpkgs attribute into scope; enables `${attr}` in directive
arguments), `COPY <src> <dst>` (verbatim sibling file — docker semantics, never substituted),
`FILE <dst> <<EOF` (inline, `${…}`-interpolated), `LINK <dst> <target>`. D55 later removed
`SCRIPT`; real scripts are copied and invoked through an explicit package shell.

Service directives (compile to spec v2): `SERVICE <name>`, `EXEC`, `SETUP`,
`ENV NAME=value [secret]`, `ENV NAME required`, or bare `ENV NAME` for an optional unset value;
spaces around `=` and defaults combined with `required` are parse errors. `PORT name = $VAR` (env
form) / `PORT name = 8080` (value form), `STATE` `CACHE`
`LOGS` `CONFIG` `RUNDIR` (role dirs, D11-narrowed paths), `JIT`.

Interpolation rule: `${…}` (build-time) lives in directive arguments and in `FILE`
heredoc bodies (`$${…}` escapes to a literal); `COPY`'d files are always verbatim; `$VAR`
(runtime env) only in EXEC/SETUP. Native projected paths (D22) remove the *need* for
interpolating file contents — heredocs merely retain the option. There is no RUN, deliberately.

Determinism: `cix build [dir] [-t ref]` compiles Cixfile → nix expr → store item. nixpkgs is
pinned in `Cixfile.lock` (rev + narHash; created on first build, `--update-lock` to roll).

## Part 4 addendum — beyond v1 (design pending)

- Positioning per D16: not essential to baseline value, but where the non-nix audience is won.
  Sequenced last; `composix.lib.withSpec` (nix helper) lands much earlier as the nix-native rung.
- Two halves: BUILD (project → store item) and SERVICE (→ spec). SERVICE half is settled by the
  part-2 schema; blocks compile to `cix-manifest.json`.
- BUILD half MVP position 🔶: no general imperative RUN steps (impurity). Recognize ecosystems
  (cargo/pnpm/uv/go + their lockfiles → nixpkgs deterministic builders); plus the D4 escape hatch
  (write nix yourself). Discipline: Cixfile stays sugar over a small set of blessed builders —
  never a general build system. General build steps: later, if ever.

## Part 5 — networking (direction set 2026-07-28; implementation is compose-era)

- ✅ D23 — **the composite is the network boundary.** Per-*service* netns + bridge/NAT + DNS
  (the docker model) is rejected: it isolates at a boundary finer than the trust boundary and
  imports docker's messiest machinery. Instead, each composite gets ONE network namespace:
  services inside share a loopback (own 127.0.0.1:5432 per composite — collision-freedom and
  privacy in one move; intra-composite addressing is `localhost:port`, DNS becomes unnecessary
  rather than unimplemented). Publishing to the host edge is an explicit per-port compose
  decision (socket-activation preferred, `systemd-socket-proxyd`/DNAT otherwise).
- ✅ D24 — **kernel-enforced port declarations now** (spec v3, independent of the rest):
  `SocketBindAllow=`/`SocketBindDeny=` compiled from the declared ports — a service cannot bind
  what it didn't declare. Cheap, no design dependency.
- ✅ D25 — **the capability tier**: wherever apps permit, intra-composite wiring prefers unix
  sockets in shared runtime dirs + socket activation for published ports (`PrivateNetwork=yes`
  services that never touch an IP stack). Framing (Mathijs): this is *pure capabilities* — an
  fd is the original capability: possession is authorization, unforgeable, delegable; the
  network disappears as ambient authority. Bonus: <1024 without capability grants, on-demand
  start, zero-downtime restarts.
- 🔶 D26 — **networks as named, realization-pluggable objects** (compose surface, like docker
  compose `networks`): a network = name + stable subnet + IPAM state (persisted in the
  composite lock/state, never ephemeral). A composite attaches via one veth + address per
  network into its single netns. **Per-service membership is enforced, not plumbed**: services
  share the composite netns, and unit-level cgroup-eBPF (`IPAddressAllow=<its networks'
  subnets>` + `IPAddressDeny=any`) restricts each service to the networks it declares.
  Local realization: bridge (networkd-managed, `IPMasquerade=` for egress). Multi-host
  realization later: the same name+subnet semantics over wireguard/vxlan/host-tunnel — the
  design constraints that keep this compatible are: stable addressing from persisted IPAM, no
  link-local/broadcast reliance, address-keyed ACLs (they carry over unchanged).
- 🔶 D27 — **service→service permission (defence in depth)**: compose grows a declarative
  `talks-to` edge; it compiles to the strongest available mechanism and reports which:
  (a) D25 tier: true per-service capabilities (fd possession, unix-socket fs-perms +
  `SO_PEERCRED`); (b) IP tier: address-keyed allow-lists — honest granularity today is
  *composite-level* (peers within one composite share the veth address, so "A → B's postgres
  only" is not IP-expressible without baroque per-service addressing); port-aware rules via
  systemd's `NFTSet=` integration are the candidate refinement. No pretending: coarse
  enforcement is reported as coarse.

## Part 3 amendment — the compose language question, dissolved (2026-07-29)

- ✅ D28 — **compose's canonical form is machine format: `compose.json`.** Strictly schema'd
  (unknown keys rejected, JSON-path source-spanned errors), validated semantically
  (`cix compose check`), consumed deterministically, diffable (`cix compose diff` between
  generations). Human authoring is a *generator* concern outside cix's core: trivial
  composites are hand-written JSON; real ones are config-as-code in any language emitting
  compose.json against the published JSON Schema (precedent: the private fleet repo's
  `infra/config/config.py` + `generate.py` — computed placements, gitignored rendered
  artifacts, regenerate-before-use behind a content hash; at production complexity compose
  config is program output, not a document). The k8s lesson re-read: machine-format manifests
  were right, leaving generation to text templating (Helm) was the mistake — we bless
  generation-as-code from day one and the data format never grows a template feature.
  `cips/draft/compose-syntax.md` stands as the encoding archive: TOML (its recommendation) and the
  Cixfile-DSL become *candidate sugar encodings*, evidence-gated on people actually
  hand-writing composites at scale. Symmetry worth naming: Cixfile : item :: your-generator :
  composite — human languages at the edges, JSON contracts throughout.

## Building now (updated 2026-08-05)

The adopted board is built through CIP-109: the corpus stands at 28 receipted cases on the
CIP-91/92 canon with the ribbon vocabulary live; the structural round landed (build_chain
4,369 -> 2,059 live with Workspace/Memo owners, runtime.rs 290 with target/app/manager strata,
doc harness split, guardrails enforced in the gate); dev-speed shipped (contract-keyed VM
selection — build-subsystem diffs 0/14 scenarios, lock subtree aggregation, scratch lifecycle
with liveness-guarded sweeping); probes are URL-shaped; STOPSIGNAL/stopTimeout exist; the k8s
axis has a drafted teaching contract and wave design.
The honest frontier is the LANGUAGE EPOCH awaiting adoption (nodes-and-edges: argv-first
steps + LET/WITH edges + heredoc-only shell; phase-blocks; build-args) with
fmt-key-neutrality as its prerequisite, the pnpm ecosystem-fetch wall (five exhibits),
CIP-103's context/sandbox + FETCH-state legs and CIP-104 crate strata, k8s wave 1 behind its
adoption call — and unchanged behind those: the phase-2 closed-root flip, D26/D27 named
networks and `talks-to`, the publish era, and the reconciler.

- ✅ D29 — **spec v3**: (a) `listeners` field per the dstyle proposal — an activated-listener
  contract *distinct from* `ports` (fd-inherit means NO IP-socket grant; `FileDescriptorName=`
  from the listener name; `cix run -p name=addr` binds it via a transient `.socket` unit with
  explicit `Sockets=`/`Requires=` wiring); (b) D24 built: `SocketBindAllow=`/`SocketBindDeny=`
  compiled from declared ports (kernel-enforced declarations); (c) the unit generator exposed
  as a library API (naming scheme + extra properties injectable) so compose can compile
  services without going through `cix run`. Version gating per D15: new fields ⇒ `cixManifest: 3`,
  runner accepts 1–3.
- ✅ D30 — **compose v0 scope** (deliberately lean; each deferral recorded in the ledger):
  IN: `compose.json` schema + `cix compose check` (D28), resolve→`cix.lock` (local tags +
  qualified refs via index), per-composite generation built as a store item + **nix profile
  per composite** (atomic upgrade/rollback), activation via `/etc/systemd/system` links +
  daemon-reload + restart-changed, `cix up`/`down`/`rollback`/`compose diff`,
  `cix-<comp>-<svc>.service` in `cix-<comp>.slice` under `cix-<comp>.target`, env overrides,
  listener bindings, **unix edges** (per-edge groups, the proven dstyle mechanism), update
  policy per service (`pin`/`track`).
  OUT (v1+): composite netns (D23 — v0 is host networking, honestly), scale/replicas,
  socket-proxy publish (v0 publishes via ordinary declared ports), health wiring, secrets
  (`LoadCredential`), resource limits, the reconciler daemon (v0 `cix up` is one-shot).

- ✅ D32 (2026-07-29) — **`PKG` is scrapped; interpolation goes flake-shaped.** `PKG` only ever
  bound a name for `${…}` — it was double bookkeeping with a manifest *suggestion* that was
  never authoritative, because nix's truth is: **references define dependencies, declarations
  don't** (the closure is the only non-lying manifest; `nix path-info -r`). Instead,
  interpolation gets the `pkgs.` namespace bound to the locked nixpkgs, with arbitrary
  attribute paths for free (`${pkgs.postgresql}/bin`, `${pkgs.python3Packages.x}`). Bare
  `${name}` without a namespace is an error suggesting `${pkgs.name}`. Future lock inputs
  beyond nixpkgs arrive as sibling namespaces — flake-inputs semantics without grammar
  changes.
  **Amendment (same day, Mathijs): `FROM` returns, honestly.** An unbound `pkgs` is itself
  ambient magic, so every Cixfile REQUIRES a `FROM <flakeref> [AS <name>]` heading that binds
  a package universe to a namespace — and `AS` is REQUIRED: no default binding, the name is
  always written. **Registry names are refused** (the flake registry is ambient host state —
  the same sin as docker.io, refused in D12): the canonical spelling is a full flakeref,
  `FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs`. Moving branches are fine — that is the
  tags philosophy: refs may move, `Cixfile.lock` pins (rev + narHash, `--update-lock` rolls
  deliberately). If verbosity ever hurts, the evidence-gated sugar is a cix-owned documented
  constant table, never the host registry. **`WITH` (nix's `with pkgs;`) is rejected**: it
  breaks name provenance (ambiguous across universes) — every name keeps a visible origin.
  Package *customization* (`override`, feature flags) is deliberately not Cixfile territory:
  that is build-land, served by the `.nix` escape hatch (D4). `Cixfile.lock` is keyed per input
  (`--update-lock [name]`). Docker muscle memory restored with a truthful meaning: not "which
  layer do I inherit" but "which package universe do I draw from".
- ✅ D31 (2026-07-29) — **the `PATH` directive; LINK is for assets, not bins.** Cixfile gains
  an explicit, item-level `PATH <dir>…` declaration (repeatable; order = search order; no
  implicit PKG⇒PATH magic — Mathijs's call, consistent with the minimal-magic budget). Two
  mechanics: (1) *build-time resolution* — a bare argv[0] in `EXEC`/`SETUP` resolves against
  the declared PATH to the real absolute store path, written into the spec (this sidesteps the
  systemd trap that ExecStart name-lookup uses a fixed compiled-in search path, not the unit's
  `Environment=PATH`; and it invokes real binaries at their real prefixes, dissolving the
  symlink prefix-inference bug class); (2) *runtime* — the same dirs become a generated
  `env.PATH` default so scripts call bare `initdb`/`postgres` — zero spec-schema change, it's
  an ordinary env var (D20b: item territory). The LINK-for-executables convention is dropped;
  `LINK` remains for non-executable assets (`mime.types`, `LD_PRELOAD` libraries, share
  trees). `EXEC ${pkg}/bin/x` stays valid for the trivial single-binary case.
  **Addendum (2026-07-29, Mathijs): the toolbox-LINK exception is refused; rationale
  refined.** The proposal to re-admit relative LINK for debug-shell tooling is dead: the
  manifest already carries the generated runtime PATH, so `cix exec` (the docker-exec
  analogue) reconstructs the service's environment from the manifest — tools are reachable in
  a debug shell without ever being shimmed into the item, and ad-hoc tool injection is just
  prepending another store path to that PATH (a future `exec --with <pkg>`), no image surgery.
  Residual cases don't exist: hardcoded paths and shebangs need *absolute* LINK anyway.
  Also correcting the D31/D32 record: the argument against `PKG`-style implicit `/bin` was
  never "new magic" — `bin/` magic is *existing* nix magic (profiles, `nix shell`, buildEnv).
  The real arguments: it *borrows the convention's name recognition without its mechanism* (a
  Cixfile builds no symlink-forest profile, it generates a PATH string — implying `/bin`
  gestures at machinery that isn't running), and it *covers only the bin half* (`share/`,
  `conf/` still need explicit paths), so PATH survives alongside it and PKG is a second
  spelling for one case of it. Less magic stays better magic, for these reasons.

- ✅ D33 (2026-07-29) — **the file is `$out/cix-manifest.json` (superseding D8's
  original file name), and baking it is nix-philosophically correct.** Terminology: *spec* = the
  declaration schema/language (spec v2, v3 …); *manifest* = the concrete instance baked into
  an item. Naming: OCI's analogue of this file is the image *config*
  (`vnd.oci.image.config.v1+json`; OCI "manifest" is the registry-side layer descriptor), but
  "config" here would collide head-on with operator territory (D20b) — and "manifest" is what
  the docker world colloquially calls the baked metadata, matching our own narrative ("a
  closure is an image with the manifest ripped out; composix puts it back"). OCI terminology
  mismatch footnoted in docker.md. Its JSON version key is `cixManifest`.
  **Why baked, when nix refuses to bake `meta`:** nix's eval-time `meta` is genius *scoped to
  non-load-bearing catalog data* (descriptions, licenses churn without rebuilds). Nix's deeper
  principle is that the hash covers everything that affects behavior — and the manifest is
  load-bearing interface: if `exec` or a port grant changes, a new store path is exactly
  right. Nix itself bakes load-bearing, tool-consumed metadata into `$out` when consumers
  shouldn't need the producer's expression: `nix-support/setup-hook`,
  `nix-support/propagated-build-inputs`. **The manifest is a nix-support file with a
  schema.** The gap composix fills: in nix, learning an artifact's runtime interface requires
  evaluating its producer's expression (and the run-contract layer — NixOS modules — is
  host-coupled); composix makes the interface travel with the closure, so `cix pull && cix
  run` needs no eval and no nixpkgs on the consumer box. The eval-time object still exists
  and stays authoritative for *production*: the Cixfile + `Cixfile.lock` (or the `.nix`
  escape hatch); `drvPath` in index entries remains provenance. Identity-for-distribution is
  the digest-addressed artifact (D5), not the expression — tags point at store items, and
  "cixpkgs as a source registry" is explicitly refused (it would re-solve nixpkgs's problem
  and break tagging locally-built results). Accepted cost, deliberately: a wrong port
  declaration can't be fixed without a new store path — for a capability grant (D20a),
  editable-in-place would be the bug; closure sharing makes the rebuild ~free.

- ✅ D34 (2026-07-29) — **`cix exec` + `cix debug`: two verbs, derived from user stories, not
  one knob-laden exec.** The stories resolve every design tension:
  *Story A (spec author, service won't start / "would the sandbox permit X?")* — there is
  nothing to join; served by **`cix debug <installable-or-ref>[#service] [--user] [-- cmd]`**:
  a fresh transient unit built through the D29c generator library with the *identical* sandbox
  (projection, seccomp, caps, dirs), but the entrypoint is an interactive shell
  (`systemd-run --pty`, `--collect`, `cix-debug-<svc>-<nonce>`) or a one-shot `-- cmd`. Full
  confinement fidelity for free — which is the point: the question is what the sandbox
  permits. Works with zero running instances (docker-debug's stopped-container story);
  `--user` is the D13 loud degraded variant. Listener sockets are not wired (a debug shell
  inherits no fds).
  *Stories B/C (operator: admin against a live service — psql, migrations, state-dir surgery;
  live inspection — procs, sockets, strace)* — served by **`cix exec <unit-or-service>
  [--root] [-- cmd]`**: join whichever of the *running* unit's mount/pid/net/ipc/uts namespaces
  differ from the caller's (nsenter-style — systemd has no first-class join,
  `JoinsNamespaceOf=` never shares the mount ns). PID is host-shared with today's generator
  *(superseded by D36)*;
  network is host-shared for a port-declaring service and private when the spec denies network.
  Environment comes from the unit's recorded `Environment=` (including any generated PATH and
  `-e` overrides), default identity = the service's DynamicUser uid (story B: files created
  during data surgery must carry the ownership the service expects; `--root` is the explicit
  escape, and root-in-namespace is real root — no userns). Deliberately **no synthetic
  seccomp/cgroup confinement on the joined shell**: the service's `@system-service` filter
  would block strace/tcpdump — the operator's tools working is story C's point. The grant
  confines the service (D20a); exec is operator surgery (D20b). Documented loudly.
  Shared mechanics: command and shell lookup use recorded/generated PATH followed by the
  operator fallback `/usr/bin:/bin`, then a clear error; future
  `--with <pkg>` = prepend a store path to that PATH (D31 addendum). Consciously refused:
  shell-into-container as a pet-server workflow — transient units, `--collect`, and immutable
  items already lean against it; the ledger says so rather than staying silent.

- ✅ D35 (2026-07-29) — **part-1 ledger resolutions** (Mathijs's docker.md review, bundled):
  (a) *Signing, scoped honestly:* content signing is ✅ today (nix path signatures are
  content-bound — trust travels with the signature regardless of which cache serves the
  bytes; `trustedKeys` in entries). What is NOT signed is the **entry** (the tag→path
  binding): trust there = TLS to the origin, which is docker-without-DCT parity (DCT/Notary
  adoption was ~nil; the ecosystem moved to sigstore). Entry signing + key lifecycle
  (rotation, revocation, delegation, policy) is ⏳ publish-era — it becomes *necessary*
  exactly when third parties sit between origin and consumer.
  (b) *Image lifecycle = tag lifecycle:* the tag is the object, store items are shared
  substrate, nix GC is the collector (untagged = unrooted = collected). No dangling-image
  objects, no prune verb, no `cix gc` machinery — at most a "run `nix store gc` to reclaim
  disk space" hint in relevant output. Auto-gc refused: `nix store gc` is store-wide and a
  shared nix host may root things cix doesn't know about.
  (c) *Mirrors: don't build them.* Bytes: substituters ARE the mirror surface — an entry may
  list multiple locations, and content-bound signatures make untrusted mirrors harmless
  (they can refuse to serve, never lie). Index availability: D18 made the API plain
  content-negotiated HTTP GET, so ordinary HTTP infrastructure (CDN, caching proxy, DNS
  round-robin) is the availability story — no cix feature. Independent index
  *redistribution* (re-serving entries under another name) is ⏳ publish-era, gated on
  entry signing per (a).
  (d) *`cix inspect` 🔶 designed, build later; `cix du` parked.* One verb, two worlds:
  `cix inspect <ref>` → store path, narHash, per-system outputs, the resolved manifest,
  closure size, signatures/keys, upstream, drvPath-if-present; `cix inspect <unit>` → state,
  MainPID, exit cause, the *effective* generated properties (the D20a mapping), port/listener
  bindings, host paths of the role dirs. JSON-vs-human default is an implementation-round
  taste call. `cix du` (per-tag self/shared closure breakdown; prior art `nix path-info -S`,
  nix-du; docker analogue `docker system df -v`'s SHARED/UNIQUE columns) is parked until
  wanted.
  (e) *`docker manifest`: no verb, ever.* It exists because docker's multi-arch is a bolted-on
  registry object needing create/annotate/push; our entries are natively per-system (D14).
  The residual gap is visibility only: a systems column in `cix ls -l` and the inspect
  output.

- ✅ D36 (2026-07-29) — **`PrivatePIDs=yes` becomes a generator default** (system mode,
  systemd ≥ 257): every service gets a real PID namespace — private process view (host
  processes invisible and unsignalable even at equal uid), the unit's main process is ns-PID
  1, namespace dies with it. Docker-parity isolation, honest `ps`, and `cix exec`'s pid join
  becomes meaningful. Fallback: where the property is unsupported (older systemd, user
  manager) it is dropped loudly via the existing degraded-mode path (D13 pattern). The
  accepted trade-off, documented: the app inherits PID-1 duties (zombie reaping, explicit
  signal handlers) — a non-issue for master/worker daemons (nginx, postgres reap anyway),
  and any example that misbehaves as PID 1 is *spec-boundary evidence* for a future
  init-shim grant in the spec (docker `--init` analogue), not something to patch around.
  Proven by the dogfood/VM gate, not by assertion.
  **Empirical addenda (2026-07-29, at merge):** (1) systemd realizes PrivatePIDs for a
  DynamicUser service via an unprivileged user namespace, so hosts that restrict those
  (stock Ubuntu's `apparmor_restrict_unprivileged_userns=1` — observed live on the dev
  host) take the loud fallback; the private-pid path is proven in the NixOS VM gate
  (`/proc/1/comm` probe). Real-world coverage therefore varies by distro userns policy —
  documented, not hidden. (2) First init-shim datapoint, exactly as predicted: node serves
  fine as ns-PID 1 but ignores SIGTERM (unit needs SIGKILL); recorded as spec-boundary
  evidence for a future init grant, no workaround applied.

- ✅ D37 (2026-07-29) — **honest examples: the pack/compose/build layout, the withSpec rung
  built, and the BUILD surface split in two.**
  (a) *Layout:* `examples/pack/<name>/` (the service items; Cixfile is the canonical form),
  `examples/compose/` (composites that consume the packs **via tags** — the compose examples
  become an integration test of the tag→resolve→lock chain), `examples/build/` (build-story
  projects; `buildshape` becomes `build/proj1`). `dstyle/` stays in place as a design-era
  archive, labeled as such.
  (b) *Honesty fix (Mathijs):* the hand-rolled `runCommand` default.nix files (heredoc
  scripts + hand-written manifest JSON) are neither what a docker refugee writes (Cixfile)
  nor what a nix-native writes — they are dogfood-era artifacts and unfair for
  apples-to-apples. D16's middle rung gets built: `composix.lib.withSpec` (attach a manifest
  + mount links to an existing derivation), demonstrated on one simple pack as the idiomatic
  `.nix` form; duplicate default.nix files are deleted where the Cixfile is canonical.
  (c) *BUILD split (Mathijs): tooling integration ⊥ multi-stage.* Increment 1 = single-stage
  tooling integration: `BUILD rust` with Variant A's fixed crane semantics but NO
  stage surface — proven by `examples/build/projB`, a plain single-binary rust service.
  Increment 2 (separate, later) = the multi-stage machinery (`STAGE`, `COPY --from`,
  `OUTPUT`, `BUILD pnpm`) proven by `build/proj1`. Variant B stays behind its evidence bar
  (cixfile-build.md unchanged).

- ✅ D38 (2026-07-29, position; **promoted same day — spike gate passed**, see addendum) —
  **the RUN hypothesis: traced read-closures as input declaration.** What nix-land build tooling
  (crane/naersk/uv2nix — the shim industry) compensates for is a missing primitive: *run a
  command and use its observed read-closure as its dependencies*. In nix this is unusually
  well-defined — legal reads are immutable store paths, so an observed read-set IS a
  closure — and nix already trusts observation on the output side (runtime references are
  discovered by scanning, not declared); the primitive extends the same empiricism to
  build inputs. Soundness conditions: no network in the sandbox (lock-derived fetch FODs
  cover hash-complete ecosystems) and determinism (verified by sampled rebuilds); under
  those, cache semantics are Rattle/BuildXL-shaped: memo table `hash(command + traced
  input hashes) → content-addressed output`; any traced path absent from the offered
  closure ⇒ miss ⇒ re-run + re-trace. This is retroactive-impossible in input-addressed
  nix but is exactly the ca-derivations shape (floating outputs + a shared, signed
  realisation table — observed keys instead of declared ones). A composix prototype needs
  no nix-core changes: our own sandbox (the cix-run/debug machinery) + read tracing
  (fanotify/FUSE) + `nix store add` + a memo section in `Cixfile.lock`; sharing later via
  the index, meeting D35's entry-signing era. Would make `RUN cargo chef cook` honest —
  the ledger's ❌ on RUN rejected *untracked* RUN. Gate to promote ✅: a spike proving a
  real `cargo chef cook` traces to a stable closure across runs. Until then the engine
  contract (if built) is the pragmatic bridge, explicitly disposable.
  **Spike evidence (same day; `.dev/spikes/run-trace/REPORT.md`, independently re-verified
  at merge):** gate PASSED — `cargo chef cook` read an identical 37-path store closure
  across forced runs; go (5), pnpm (34), uv (13) closures equally stable; memo hit/miss
  semantics behaved for all four; the harness stayed fully ecosystem-agnostic (the golden
  path held — all cargo/go/pnpm/uv knowledge lived in the example projects). Honest
  residuals for productization: (1) *output* nondeterminism exists even where closures are
  stable (cargo incremental session dirs, uv's embedded timestamps) — sampled-rebuild +
  realisation policy needed; (2) "offline" ≠ "read-only cache": pnpm/uv need writable cache
  snapshots (and pnpm 11 needs `--trust-lockfile` to be truly networkless); (3) non-store
  inputs (source tree, cache snapshots) need generic fingerprinting — whole-tree hashing is
  sound but over-invalidates; (4) strace is prototype-grade — a product observer
  (fanotify/FUSE/eBPF) must be auditable enough to sign and share. Tracing overhead
  1.3–1.7×. Composition across RUN steps works (prior output seeded as a lower layer).
  Consequence: the engine-contract bridge is NOT built; RUN productization is the build
  story's next design round.

- ✅ D39 (2026-07-29) — **RUN v0: the D38 primitive productized** (Mathijs: "perfect, bouw
  maar"). The four spike residuals get these answers:
  (1) *Realisation policy, tiered*: the memo needs no determinism locally (key → the output
  we got); layers (intermediate steps) need only functional adequacy, artifacts get
  reproducible-env defaults (`SOURCE_DATE_EPOCH`, `TZ=UTC`, `LC_ALL=C`, fixed umask,
  `HOME=<workdir>`) + sampled-rebuild verification; realisation *sharing* is publish-era and
  reuses D35's signing story.
  (2) *Caches & network*: non-store inputs are offered as writable snapshots (source
  fingerprint = input; overlayfs where cheap). **`FETCH` is its own directive** — the only
  network-allowed step, fixed-output: its result hash is TOFU-pinned in `Cixfile.lock` and
  verified on re-fetch. RUN never sees a network. The distinction is a promise to the
  reader, hence a keyword, not a flag.
  (3) *Granularity = COPY-scoping (docker's own answer)*: steps form a linear chain; each
  step sees exactly the workdir accumulated by prior COPY/FETCH/RUN steps and is keyed on
  its snapshot hash — `COPY recipe.json .` before the cook step is precisely why a source
  edit doesn't re-cook. Whole-snapshot hashing per step is then correct, not
  over-invalidating. Trace-observed workdir granularity: later, evidence-gated.
  (4) *Soundness by construction, observation as optimization*: the RUN sandbox mounts
  ONLY the offered closure (FROM/PATH-declared paths + their nix closures — the D22 sparse
  machinery's build-time sibling), no network, fresh namespaces. An unoffered read cannot
  happen, so the tracer is not load-bearing: **v0 ships without one** (memo key = command +
  offered closure + snapshot); strace/fanotify-based pruning of the offered set to the
  observed set is a v0.5 optimization (spike: 1.3–1.7× overhead when on).
  Surface (v0): `FETCH <cmd…>` / `RUN <cmd…>` between the FROM/PATH/COPY prelude and
  SERVICE blocks; the final step's workdir snapshot is addressable as `${build}` in
  EXEC/LINK/PATH. Supersedes D37(c)'s BUILD increments — `BUILD rust` is dead, the
  buildtool worktree stays parked as archive. Consequence for the ledger: docker's `RUN`
  row flips from ❌ to a sandboxed, memoized, lock-pinned ✅ — the universal escape hatch,
  made honest.

- ✅ D40 (2026-07-30) — **build outputs & caches** (designed 07-29, go via the 07-30 tree
  round: Mathijs, "spec het maar precies en laat het bouwen"). (a) **`ITEM` plucks**: a
  Cixfile produces one or more *items*, each declaring `TAKE <build-path> <item-path>`
  plucks — items carry declared subpaths, not the whole final workdir snapshot (kills the
  `${build}` cargo-bookkeeping bloat, and the D39.1 layer noise lives exactly there).
  `ITEM` supersedes `SERVICE` as the block keyword. (b) **`CACHE <dir>`**: advisory
  per-step cache directories that live OUTSIDE the memo key and OUTSIDE the snapshot —
  the docker `RUN --mount=type=cache` analogue; enables ecosystem-incremental builds
  (cargo target dirs, pnpm stores), soundness bounded by sampled clean rebuilds (D39.1
  policy). (c) Gate: proj1 (a real multi-item application) builds, runs, and re-builds
  incrementally end-to-end.
- ✅ D41 (2026-07-30) — **item = exactly one service; the manifest is a bare def-node**
  (Mathijs's push: a rust project building 5 services must not fuse them into one item —
  that's non-granular rebuild; the store dedupes shared closures at path level so
  splitting is free). `cix-manifest.json` v4 drops the `services` map: the file IS one
  service body (`exec`/`setup`/typed `env`/`ports`/`listeners`/`dirs`/`health`/`jit`/
  `egress`, final spelling per D48(b)). Multi-service manifests (D8-era) retire;
  compose's `service:` selector field dies. Runner accepts 1–4 per D15.
- ✅ D42 (2026-07-30) — **the compose tree: one grammar, two artifact kinds.** A *pack
  item* (filesystem + def-node manifest) is the leaf; a *compose artifact* (`cix.json`:
  children refs + `edges` + `publish` + `network`, optionally nested) is the node; both
  are store items, both taggable in the index. Instance identity = **path in the tree**
  (units `cix-<path…>-<svc>.service`, nested slices, path-keyed state) — the artifact's
  `name` is self-description only; two instances of one artifact under different keys are
  fully disjoint. The **host root is the same format in a mutable file** (+ lock), and
  day-two CLI verbs are structured edits of that file (the D28 machine-format payoff);
  only the root must live on the host, every subtree may be a ref. Full story:
  CIP-85 (cips/accepted/0085-compose-tree.md; consolidates D40–D46).
- ✅ D43 (2026-07-30) — **pod-ness is a scoped property, not a stratum.** `network: "pod"`
  on any composite claims one netns for its subtree; **nearest-pod-ancestor** decides
  each service's namespace (pod-in-pod legal; embedding never strips a sealed artifact's
  declared boundary). Crossing a boundary requires declaration: `publish` climbs one
  boundary at a time (child publish → parent scope, where it's `bind`-ed or re-published),
  `edges` (D25 fd tier) cross netns for free. Absence of pod-ness anywhere = today's
  host networking (rawdog = absence of a property; the `network: host` escape flag
  dies). Per-service **`egress: true`** (final spelling per D48(b); D20-side app
  semantics) declares outward initiation; absence = loopback-only view, zero network
  machinery for pure composites. Amends D23's fixed boundary; CIP-86 remains
  the realization paper.
- ✅ D44 (2026-07-30) — **ref/lock semantics unified on every floor** (Mathijs: pinning
  was always a root-level affair). Refs are always `name:tag` — no bare names, no
  ranges, no solver; softness is publisher tag discipline (immutable tags or
  hash-qualified refs for hard sealing). The **operative lock lives with the deployer**;
  a published composite embeds an *advisory* lock snapshot (seed + hermetic testing +
  provenance, not authority). `cix up` replays the lock; `cix up --update [edge…]`
  repins deliberately; root-side `track` = auto-repin on up (reconciler case);
  publisher-side pin/track fields do not exist. Every generation is fully pinned
  regardless — the axis only decides *who may move which pointer*. Repin runs the
  wiring-as-interface check (new artifact must still provide the surfaces the tree uses).
  **Override is not built**: evidence-gated future in cargo-`[patch]` shape, declared at
  the piercing level; unit-level piercing already exists as systemd drop-ins +
  `systemd-delta`. Rollback = generations as mechanical crash net; semantic undo = tag
  push (roll-forward). This is D32's flake-inputs move re-derived at compose level.
- ✅ D45 (2026-07-30) — **the index re-founded: names → content-addressed tag tables.**
  Per name, one store item holds the tag table (`tag → {storePath, narHash, meta}`) +
  the parent table's hash (history chain, publish-rollback, audit by construction). The
  only mutable cell is a `name → table-item-hash` map; publish = build table item +
  **CAS the name pointer** (atomic multi-tag publishes, race detection). Yank =
  publish a table without the tag — advisory by nature (crates.io semantics; bytes
  cannot be recalled, deletion stays GC per D35). Signing (D35 ⏳) collapses to signing
  the table hash per name; **auth is name-level: who may move this name**. Serving
  degrades toward substituters-for-bytes + a static `name → hash` lookup. Prior art
  convergence: git refs/objects, OCI tag→manifest, crates.io index, nix channels.
- ✅ D46 (2026-07-30) — **computable composes: publish-time expansion only.** A
  parametric compose (`my-app:$tag` ≡ compose of `my-frontend:$tag` + `my-backend:$tag`;
  `$tag` is the only variable, only in tag position) is macro-expanded by
  `cix publish my-app:v1.2.5` into an ordinary concrete artifact — resolvers stay dumb
  data readers, no interpreter in the index. Monorepo tooling on top: one verb builds
  and publishes the whole family for a version bump, one atomic table move per name.

- ✅ D47 (2026-07-30) — **the Cixfile becomes blocks and binders; zero ambient names**
  (Mathijs's redesign round, same day as D40/D41 landed — the cheapest moment there
  will ever be). The language is a graph of named blocks; every `${name}` has a
  visible binder:
  (a) **Block kinds**: `BUILDER <name>` (the workshop: COPY/FETCH/RUN/CACHE/PATH;
  memoized, advisory, may be messy; the block name binds its final workdir snapshot —
  `${build}` magic dies because you wrote `BUILDER build`), `SERVICE <name>` (def-node
  artifact, v4 manifest), `APP <name>` (executable, run-to-completion: exec/env/
  egress/dirs; NO ports/listeners/health — parser-enforced; manifest kind "app"),
  `ITEM <name>` (pure assets, no exec). D40's ITEM-as-service keyword is superseded.
  (b) **RUN is caged**: legal only inside BUILDER. **Network always pins**: `FROM`
  (universes AND sources — `FROM . AS src` binds the local source tree, killing the
  last ambient root; remote `FROM github:… AS src` builds foreign sources, flake-input
  semantics; `.` is not lock-pinned — it IS the input, like a flake's own tree) and
  `FETCH` (two forms: top-level `FETCH <name> <cmd>` in an empty workdir — a pinned
  ingredient whose memo key never moves with source edits; in-builder `FETCH <cmd>` as
  chain step for lockfile-context fetches).
  (c) **COPY is the one mover; TAKE dies**: sources are binder-rooted (`${src}/…`,
  `${build}/…`, `${webvault}/…`, `${pkgs.x}/…`) — with ONE sugar (amendment, same
  day, Mathijs): a bare relative source stays legal as the implicit Cixfile-directory
  context, exactly docker's build context ("niet zo mooi, maar docker heeft het ook" —
  the Cixfile is the adoption bridge, D16; the context is ambient in spelling only,
  not in content: it is content-addressed per build). `FROM . AS src` is the optional
  explicit spelling of the same thing; remote sources always need an explicit FROM
  binder. Destinations are ALWAYS subject-relative (inside
  the block's own artifact/workdir — one destination per block, grammar-enforced;
  writing outside your own artifact is inexpressible). Multi-stage = multiple BUILDERs
  chained via `COPY ${prev}/ .` — no new mechanism.
  (d) **CACHE and PATH become builder-scoped** (each builder its own toolchain/caches);
  CACHE survives the redesign — cache-safety is app-semantic knowledge (D20 line),
  never inferable. Item-relative PATH in artifact blocks stays (D31).
  (e) **Doctrine, now grammar**: workshop (BUILDER: memoized, sandboxed-offline,
  nondeterminism tolerated per D39.1) vs shipping dock (artifact blocks: pure assembly
  from bound sources, deterministic by construction, sampled clean rebuilds as the
  bridge check). Reference rule: backward only; cycles grammatically impossible.
  Deliberately deferred (each behind its own forcing example): APP's timer form
  (corpus: CronJob/Renovate row), APP as compose lifecycle hook (helm-hook/migration
  row), build secrets for FETCH (playwright row). Bazel positioning noted for the
  ledger: composix is the cage, not the compiler-orchestrator — ecosystem tools keep
  their own fine-grained incrementality (CACHE), correctness comes from
  sandbox-by-construction, not declared deps.

- ✅ D48 (2026-07-30) — **feedback-round resolutions, bundled** (Mathijs's review of
  D40/D43 and the six corpus-demand specs):
  (a) **CACHE redefined, kept** (D40 amendment): memo keys on inputs, so hits never
  need cache and warm state cannot cause wrong reuse — the danger is only a dirty
  workspace on a MISS (ghost files from deleted sources make the snapshot key a lie,
  and sampled rebuilds catch that only probabilistically). CACHE is therefore not a
  cache feature but **the declared exception to snapshot semantics** (outside key,
  snapshot, and items); everything undeclared keeps meaningful keys. Implicit
  whole-workdir warmth is rejected.
  (b) **`egress` returns** (D43 amendment): prior work is unanimous at the
  workload-policy layer — k8s NetworkPolicy (`policyTypes: [Ingress, Egress]`),
  Cilium/Calico, and systemd itself (`IPEgressFilterPath=`); "outbound" is the
  cloud-console word. Fundamental: ingress/egress is the symmetric pair and our
  publish/bind side IS ingress. Rename lands as a micro-round after track/blocks
  (manifest field + D47 directive become `egress`/`EGRESS`).
  (c) **Health = an edge to a consumer, not a property** (k8s lesson: probes are
  named by consumer — liveness→restart, readiness→traffic, startup→boot-gate; deep/
  business health is application-layer, explicitly out of scope). Our consumers:
  `cix up` convergence, restart policy (default report-only, per-service opt-in),
  dependent ordering (readiness ≈ ordering; no traffic layer). Manifest declares the
  probe; compose declares consumers.
  (d) **Durable data ownership = declared identities; one registry decision covers
  hostbinds AND shared edges**: host-bound state requires a declared static user
  (`--user` mode dissolves the problem — everything is the invoking user); shared
  persistent edges require a stable group. Both come from a small cix-managed
  identity registry (name→uid/gid, profile-like cell). Operator-precreated identities
  remain expressible; cix-allocated is the default position.
  (e) **House principle recorded** (from the timers call): *use systemd as
  transparently as possible; build only at a real impedance mismatch.* Timers =
  raw `OnCalendar` accordingly.
  (f) **Hooks shrink to nearly nothing** under (e): migrate-on-upgrade = an ordinary
  oneshot APP unit with `After=`/`Requires=` ordering — content-addressing makes it
  re-run exactly when its app version changes (restart-changed sees a changed
  ExecStart store path), for free. `ExecStartPre` covers the chain-style cases
  natively. The only genuine mismatch, abort-before-switch (old generation keeps
  serving on hook failure), is deferred until the native shape proves insufficient.

- ✅ D49 (2026-07-30) — **netns-round resolutions** (Mathijs's read of
  the netns paper, now CIP-86 at cips/accepted/0086-netns.md; closes that
  paper's open decisions):
  (a) **Egress is a leaf/manifest property with a compose-level per-child override**
  (`egress: true|false`), because capability need is app knowledge but *usage* is
  instance knowledge (the same app does or does not egress depending on how you
  deploy it — Mathijs's non-static observation). Tightening (declared, denied) is
  silent; loosening (undeclared, granted) is LOUD in `cix compose check` — granting
  what the artifact did not ask for is always said out loud. Same
  manifest-default/compose-override pattern as env.
  (b) **Publish fallback = proxyd-in-netns only for v1**; DNAT is not shipped
  (firewall-interaction baggage, no demonstrated need).
  (c) **Egress addressing: fixed cix-owned private range with per-composite subnets
  persisted in composite state** (IPAM survives rollback; rollback restores units,
  not leases). Address-keyed enforcement (D26/D27) requires stable addresses;
  link-local remains rejected per D26 multi-host constraints.
  (d) **Naming stays `cix-<comp>-netns.service` + `/run/netns/cix-<comp>`** — the
  prefix is collision hygiene on shared /run/netns ground (libvirt, hand-made netns)
  and consistent with `cix-*` units. Mathijs's drop-the-prefix suggestion is noted
  for reconsideration if the names ever chafe.

- ✅ D50 (2026-07-30) — **the ITEM block is dropped** (Mathijs: "zo nikszeggend dat
  je zijn betekenis compleet uit context moet halen"; and the evidence agrees — it
  shipped in D47 and zero examples use it). The language is three block kinds:
  BUILDER / SERVICE / APP. Assets consumed within a Cixfile are COPY'd into the
  artifact that uses them. If a standalone content-only artifact ever earns its way
  back (published dataset, webroot, config tree), it re-enters evidence-gated under
  the name **ASSETS** — not ARTEFACT (already our generic word for any published
  store thing) and not STORE (already the nix store). Removal lands as a
  micro-round after track/polish.

- ✅ D51 (2026-07-30) — **Cixfile ergonomics from the tour read-through** (Mathijs):
  (a) COPY of a directory is the preferred form (`COPY ${src}/rust/ .`);
  enumerate-COPY is for deliberate memo-granularity only (chef-style manifest-first)
  and says so in a comment. Structural globs (`**/Cargo.toml`) are noted-not-built,
  evidence-gated — the chef pattern already covers the known need. (b) Directive
  lines accept `\` continuation, and RUN gains a heredoc form (`RUN <<EOF … EOF`;
  body is part of the memo key) so workshop scripts stop being one-liners. Tour
  regrouping into chapter pages + the spec→manifest vocabulary sweep ride the same
  track (.dev/specs/track-tourbook.md).
  **Addendum (2026-07-30, Mathijs unrolling a chained FETCH): the heredoc is
  deliberately asymmetric — RUN yes, FETCH no.** A long single fetch is served by
  `\` continuations; multiple commands in one FETCH are the anti-pattern (one
  coarse pin instead of per-step pins), so the missing heredoc is a nudge to
  split. Unrolling also tends to reveal hidden structure: `git checkout <sha>`
  after a clone is OFFLINE work — a RUN, not a FETCH.

- ✅ D52 (2026-07-30) — **two directive-consistency fixes** (Mathijs's README read):
  (a) service-block `CACHE` (the D11 role dir) renames to **`CACHEDIR`** — kills the
  collision with D40's builder `CACHE`, and makes the role-dir directive family
  mirror systemd's own field family one-to-one (`CACHEDIR`→`CacheDirectory=`,
  `RUNDIR`→`RuntimeDirectory=`, …; the D48e transparency principle applied to
  naming). Bare `CACHE` stays exclusively workshop vocabulary.
  (b) **`LINK` argument order flips** to `LINK <target> <linkpath>` — the old order
  (linkname first) contradicted BOTH `ln -s TARGET LINKNAME` and COPY's
  source-first convention; the new order restores both muscle memories
  ("where from, where it lands", uniform with COPY). Hard flips, migration-grade
  errors, ride track/tourbook.
  **Addendum (2026-07-30 session close, Mathijs): `STATE` → `STATEDIR`** —
  completing the family consistency D52 started (mirrors `StateDirectory=` as
  CACHEDIR/RUNDIR mirror theirs). Sub-question resolved (2026-07-31, Mathijs:
  "eens"): LOGS and CONFIG follow — `LOGDIR` (mirrors `LogsDirectory=`) and
  `CONFIGDIR` (mirrors `ConfigurationDirectory=`); the whole role-dir family
  now spells systemd's names. Directive-level flip only, manifest role keys
  unchanged (same as the STATE→STATEDIR flip). Rides a micro-round after
  track/argvenv.

- ✅ D53 (2026-07-30) — **Cixfiles get `#` line comments** (Mathijs, reading the
  corpus whoami pair: none of the wild-corpus Cixfiles could explain themselves).
  Full-line comments only (a line whose first non-whitespace char is `#`);
  end-of-line comments are deliberately NOT parsed — RUN/FETCH bodies are shell,
  where `#` already belongs to the shell (and works naturally inside D51 heredocs).
  Corpus note recorded with it: dual receipts prove *functional* faithfulness, not
  version parity (Dockerfile-pinned version vs nixpkgs-rev version); a version-parity
  probe is a loss-curriculum candidate for later rounds. Implementation: micro-round
  after track/tourbook (parser + docs + a commented example).

- 🔶 D54 (2026-07-30) — **artifact annotations: designed, deliberately unbuilt**
  (Mathijs spotting OCI LABELs in the corpus). The OCI label family splits:
  provenance labels (source/version/revision) are SUPERSEDED here — the lock and the
  closure are hash-covered provenance, hand-typed versions routinely lie; display
  labels (title/description/url/docs) are a real unmet need whose consumer is the
  serve/explore surface (D18) and `cix inspect --human`. Shape when evidence
  arrives: an `annotations` map in the manifest (baked, hash-covered, D33 reasoning)
  via a Cixfile directive (`META`, or `LABEL` for muscle memory), rendered by
  inspect + serve pages. Gate: build it when the explore surface gets real users;
  until then converters drop LABELs (migrate prompt says so).
  **Field design from the prior-work sweep** (OCI annotations; ghcr's load-bearing
  `source`; Renovate reading `source` for changelogs; Docker Hub ignoring
  annotations; artifacthub needing its own richer namespace; Helm Chart.yaml as the
  compose-artifact analogue; nixpkgs `meta` already riding along on `${pkgs.*}`;
  provenance having moved to signed attestations — labels' provenance role is dead):
  hand-written = `title`, `description`, `homepage`, `documentation`, `licenses`
  (SPDX), `maintainers`, `icon`; derived-never-typed = `source` + `version`/
  `revision` from the lock (hash-true); `created` deliberately absent (determinism).
  Serve/export maps 1:1 onto `org.opencontainers.image.*` for tool interop.
  **In/out architecture (Mathijs's cut, multi-ecosystem survey)** — the common core
  across npm/Cargo/pyproject/nixpkgs-meta/Helm/Debian is name·version·description·
  homepage·license·maintainers·repository; registries render READMEs, not
  description fields; pkg.go.dev proves derive-don't-declare works.
  **Identity rule (Mathijs's "gortig" correction): identity flows from your own
  source, never from your dependencies** — a website served WITH nginx must not
  wear nginx's homepage; auto-inheritance from referenced packages is dead, and
  dependencies stay visible as closure/receipts: facts about what an artifact
  *uses*, never claims about what it *is*. **And "the source tree" may not be
  ambient either (Mathijs): the identity/provenance root is DECLARED —
  `META source <binder>`, at most one per artifact block.** It resolves two
  ambiguities at once: it pulls whatever identity metadata the binder carries
  (nixpkgs `meta` for a package ref — the dissolves-class one-liner; package.json/
  Cargo.toml/pyproject for a source input) AND designates that binder's lock entry
  as the receipted `source`/`revision`. No declaration = no inherited identity and
  no claimed provenance (closure facts remain). This subsumes the earlier
  `META inherit`. **Final mechanism (converged through Mathijs's rounds — the last
  one deleted the rest): cix parses NOBODY'S manifest. Extraction is workshop
  work.** A design arc worth keeping honest: we first sketched per-ecosystem
  adapters (rejected: ecosystem tables rot), then one generic `#`-selector with
  Rust-side serde for toml/json/yaml (rejected by "kan je niet gewoon uitshellen?"):
  the workshop already runs real tools. If a field's value needs extraction, the
  BUILDER computes it with jq/yq/tomlq from `${pkgs}` — sandboxed, memoized,
  snapshot-receipted — and writes a file; META references it. Every format on
  earth is supported the day its tool is, and cix ships zero parsers, zero
  selector grammar, zero Norway problem. What META accepts (final surface,
  Mathijs's `=`/blob round):
  - `META <field> = <value>` — literal, or a reference (inline fields read trimmed
    FILE CONTENTS: `META description = ${build}/output/description`; document
    fields readme/icon/license-file take the file itself into the item), or
    `${pkgs.x}#attr.path` (attrset selection via the existing eval — the
    dissolves-class one-liner; NOT file parsing);
  - `META <json-ref>` or `META <<EOF {…} EOF` — BULK: a JSON object in the
    internal schema, strictly validated (unknown field/type = error). The
    best-of-both-worlds bridge: standard converter TOOLS (a
    `cix-meta-from-package-json`, anyone's) run in the workshop and emit the
    blob — ecosystem convenience as tooling, zero parsers in cix core.
  **`SOURCE` is its own declaration, not a META field** (provenance is a different
  kind of thing than display). `SOURCE <binder>` = pure receipt: a remote flakeref
  binder yields url+rev+narHash (the flakeref IS the url); `.` yields
  **narHash-only** — a local working copy has no public URL, and reading one from
  `.git/config` would be ambient host state (the D12 class) that lies on dirty
  trees. `SOURCE <url> <binder>` = **claim + receipt**, the answer to the
  local-build URL problem: claimed canonical URL and receipted narHash stored
  SEPARATELY (claim vs fact, never conflated) — and the pair is a *verifiable
  claim*: anyone can fetch url@rev and compare hashes; inspect renders "built from
  X@rev" vs "claims origin X (verifiable)"; a future `cix verify-source` verb
  falls out for free. Authors may invent titles, never provenance — but they may
  state a checkable address. Honest cost of the parser-free design: a
  pure-assembly artifact wanting a field from its own manifest opens a
  mini-builder for the extraction, or writes the literal.
  So: **IN** (priority on conflict): (1) explicit META
  fields (literal/reference/bulk), (2) `SOURCE` provenance, (3) derived WITH
  receipts from the lock — `source`/`revision`/`version` are lock-backed
  (FROM+narHash) and non-overridable: authors may invent titles, never provenance.
  **OUT** adapters from one internal schema, final cut (Mathijs's trims): inline =
  title, description (one line), SPDX license expression, maintainers (a LIST,
  freeform strings v1; nixpkgs' structured form is the enrichment path);
  `urls` = one optional labeled map (pyproject pattern: homepage/docs/chat/…) —
  courtesy links, almost always inherited, homepage carries no special meaning for
  us beyond "upstream's front door"; by-reference = documents as paths INTO the
  item (readme, icon, license-file) — manifest and files share one
  content-addressed item, so document references are hash-covered receipts for
  free (kills npm's embedded-README two-truths wart; Cargo's license vs
  license-file split adopted). DROPPED: keywords (vestigial ceremony — modern
  discovery is full-text + social signals) and changelog (upstream's changelog is
  reachable via the receipted source, the Renovate pattern; and composix has a
  NATIVE change history — the D45 tag-table hash chain + generation diffs — which a
  hand-typed file would only impoverish). Adapters: inspect --human ·
  serve/explore HTML incl. README rendering · OCI annotation export · index search
  fields.

- ✅ D55 (2026-07-30) — **SCRIPT is dropped** (Mathijs: YAGNI — and the evidence
  agrees: zero Cixfiles use it; its only consumer was its own tour showcase, the
  D50-ITEM situation again). Its whole delta over FILE was shebang + exec bit,
  which `EXEC ${pkgs.bash}/bin/sh <path>` dissolves (sh reads the file; no exec
  bit needed); without interpolation the answer was always COPY of a real source
  file. **FILE stays** despite zero current users — its reason is architectural
  (store paths cannot be pre-written into source files) and the corpus middling
  tier will need it; re-evaluate if it is still unused by then. Tour style rule:
  chapters demonstrate COPY'd real files; FILE only where store-path-embedding is
  the actual lesson; the RUN heredoc demo stays (it is D51's own ask).
  Migration-grade parse error for SCRIPT; rides the next language micro-round.

- ✅ D56 (2026-07-30) — **`EXPECT` on FETCH: declared output hash beats TOFU**
  (from Mathijs's adminer giga-FETCH review). Optional
  `FETCH <name> <cmd> EXPECT sha256-…=` (both FETCH forms; trailing spelling
  adopted 2026-08-02): when present there is
  NO trust-on-first-use window — the lock entry must match the declared hash or
  the build fails. Safer (declared beats first-fetch trust), shorter (the manual
  `curl … && sha256sum -c` ceremony and its `bash -c` wrapper die), and it
  migrates 1:1 (Dockerfile checksums become the EXPECT value — prompt lesson).
  Without EXPECT, TOFU stays the honest default for sources without a known hash.
  Prior art: nix fixed-output derivations, exactly.
- ✅ D57 (2026-07-30) — **narrow read-keying: the wall becomes unnecessary**
  (Mathijs's round: "the output of a BUILDER is the filesystem it leaves behind;
  consumers key on what they read — and that key is narrow"). The cache/snapshot
  wall was an artifact of coarse keys; with narrow keys it dissolves. Invariants
  (the law; mechanics follow them):
  (1) **no key ever derives from workdir bytes** — step keys form a pure
  derivation chain: hash(command, closure, predecessor keys, declared source
  contents, env) — nix's own input-addressed model;
  (2) **artifact bytes come only from consumed-path records**: an artifact-bound
  `COPY ${builder}/path` keys and plucks on the content hash of exactly that path,
  recorded content-addressed in the store (memo replay materializes precisely the
  consumed paths — no whole-workdir snapshots as key material);
  (3) **workspaces are persistent overlays**: fresh staged lower (declared inputs,
  deletions included — ghost sources structurally impossible), persistent written
  upper (target/, node_modules/); **`rm -rf` of any workspace is always correct**
  (nothing keyed depends on it) — workspace GC is LRU policy, not correctness;
  (4) **CACHE is removed** — persistence is the unsurprising default, exclusion
  from keys is automatic (nothing is keyed unless read); migration-grade error;
  (5) **warm ≡ cold on artifacts**, enforced by the sampled clean rebuild with
  **per-consumed-path attribution**: a mismatch names the exact artifact-bound
  COPY line (Mathijs's detector) — staleness stays sampling-territory, now with
  the receipt pointing at the culprit.
  Cold-machine replay granularity is honest: a builder whose chain keys all hit
  materializes only its recorded consumed paths; any changed step re-runs the
  chain from staging (step-level skipping is a warm-workspace benefit — which is
  exactly how ecosystem incrementality works anyway). Whole-tree consumers
  (`COPY ${prev}/ .`) key on everything they read and pay accordingly; docs
  advise narrow. RUN-edge read-tracing (the D38/D39 v0.5 tracer) remains the
  later increment for pruning offered closures — this decision needs none of it. **Operational addendum (Mathijs, fleet
  experience): do NOT chase per-dep store substitution for build speed** — one
  derivation per crate makes cargo builds ~5x slower (why they left crate2nix for
  crane), and constant artifact-pulling is itself the slowdown; the only truly
  fast form was a custom runner with persistent /nix + ./target. That IS this
  model, formalized — so the CI story is: persistent runner workspaces by
  default, scheduled `--cold` runs as the honesty check.

- ✅ D58 (2026-07-30, arrived via the "SSL_CERT_FILE song & dance" → "isn't that
  just LINK?" → "make it generic" ladder with Mathijs) — **`IMPORT` replaces PATH:
  builder provisioning as package union.** `IMPORT <pkg-ref>…` (builder-scoped,
  repeatable) union-mounts the referenced packages' conventional subtrees into the
  build sandbox root — **declaration order = overlay priority**, generalizing
  exactly what PATH order already meant for bin resolution. IMPORT takes over BOTH
  of PATH's roles (bare-command resolution via /bin; offered-closure definition)
  plus the conventional-path gap PATH could never cover (`/etc/ssl/certs` from
  `${pkgs.cacert}` — the whole CA dance dies; `share/zoneinfo`; a working `/bin/sh`
  for tool-generated launchers). Union subset starts at **bin, etc, share**
  (nix prior art: buildEnv unions with priorities; NixOS curates via pathsToLink
  and deliberately generates rather than unions /etc — a concern that is light
  inside a build sandbox); extend evidence-gated. `lib` deliberately absent
  (nixpkgs binaries are rpath'd). **PATH dies in builders** (YAGNI-return clause:
  explicit `ENV PATH=…` covers genuine path needs; bring a keyword back only if
  that chafes). Service-block runtime PATH becomes a plain ENV declaration;
  D31's item-relative resolution for `EXEC bin/x` is untouched. **No cacert
  default anywhere**: `IMPORT ${pkgs.cacert}` is one explicit word; the
  convenience story is future sugar — a cix-published curated base tag (a
  universe artifact with common IMPORTs pre-declared), cix-owned per D32's sugar
  rule, never engine magic. LINK stays as the pincet next to IMPORT's brush for
  non-FHS-shaped outputs. FETCH outputs remain hash-pinned, so trust config stays
  availability plumbing, never an integrity input; RUN remains networkless.
  **Addendum (2026-07-31, Mathijs: "ja eens, doe maar") — `/usr/bin/env` joins
  the sandbox skeleton.** The kernel resolves shebang interpreters literally
  (no PATH lookup), and tool-generated launchers hardcode `#!/usr/bin/env`
  (every npm `.bin` wrapper — corpus forcing example: echo-server's offline
  webpack step). IMPORT can never cover it: the union populates only
  /bin//etc//share. The skeleton therefore provides exactly one extra entry:
  `/usr/bin/env → /bin/env` — an alias path, not ambient software: it
  resolves only when something imported ships `env`, and dangles loudly
  otherwise. Slope-guard, citable: **the boundary is exactly the two paths
  NixOS itself blesses on a running system** — `/bin/sh` (falls out of the
  union when a shell is imported) and `/usr/bin/env` (skeleton) — never a
  third. The patchShebangs school stays right for shipped artifacts; the
  workshop is a running environment, where the NixOS precedent applies.

- ✅ D59 (2026-07-30) — **post-r4 language round: builder ENV + EXEC argv quoting**
  (both straight from corpus round N=8 evidence; Mathijs: "doe maar").
  (a) **`ENV NAME=value` becomes legal in BUILDER blocks**: applies to all
  subsequent steps in that builder (docker's from-this-line-on muscle memory),
  plain values only (no typed default/required — that is runtime-contract
  vocabulary), participates in the chain key as declared text, and is injected as
  an export-prelude THROUGH the step shell so `$PWD`-style values expand at each
  step (the verdaccio pair's sixfold `COREPACK_HOME=` prefix noise was the
  evidence). (b) **EXEC/SETUP argv tokenization becomes quote-aware** (single and
  double quotes preserve spaces; unterminated quote = line-numbered error) — the
  r4 product finding that `nginx -g 'daemon off;'` was inexpressible. The early
  "no quoting grammar" boring-choice is hereby superseded for the two directives
  where argv fidelity is the contract.

- ✅ D60 (2026-07-31) — **`GRANT <capability>`: the capability-grant family**
  (Mathijs: "akkoord met grant voor nu"; prior-art survey against macOS
  hardened-runtime entitlements — `com.apple.security.cs.allow-jit` is literally
  our `jit` —, flatpak finish-args, snap plugs/interfaces, k8s/docker cap-add,
  OpenBSD pledge).
  (a) **Grammar**: repeatable directive, **one grant per line** (no multi-arg —
  keeps future argumented grants like `GRANT device dri`, flatpak-style,
  unambiguous without a later grammar flip). Legal in SERVICE and APP blocks
  only: BUILDER stays networkless/sandboxed by design, ITEM has no exec.
  (b) **Manifest**: a single `grants` list replaces the ad-hoc `jit`/`egress`
  booleans (version gate per D15). Per-capability semantics stay per-capability:
  `egress` keeps its D49(a) compose-level usage override (snap's declare/connect
  split is the precedent that need-declaration and instance-usage knobs compose),
  `jit` has none.
  (c) **Vocabulary**: closed, cix-owned, semantic names per D20a; each entry is
  documented as its exact systemd property delta (D48e transparency — e.g. `jit`
  ⇒ drop `MemoryDenyWriteExecute=`). Day one: exactly `jit` and `egress` — a pure
  spelling migration, hard flip with migration-grade errors (the `JIT`/`EGRESS`
  directives die). Evidence-gated queue, each behind a forcing example already in
  the corpus or ledger: `mlock` (vault's IPC_LOCK row), the net-admin corner
  (pihole — semantic split like `tun`/`rawsock` when it lands), `device <class>`
  (the docker.md `--device` deferral), `realtime`, `namespaces` (self-sandboxing
  apps), `fuse`.
  (d) **Refusals**: no `GRANT all`/privileged (D20a; ledger ❌), no raw `CAP_*`
  names (the k8s anti-model — mechanism must not leak into the language).
  Beyond-vocabulary needs are escape-hatch territory (compose operator overrides,
  the `.nix` route), never language.

- ✅ D61 (2026-07-31) — **the rootless/non-Linux user story: settled lanes**
  (Mathijs's verdicts on the docker-desktop/podman/systemd survey).
  (a) **`cix machine` is wanted unconditionally** ("sowieso ideaal voor dev en
  dit soort tour dingen"): a cix-managed lightweight NixOS VM running the real
  system systemd — full-fidelity system-mode semantics for dev on any host;
  Linux first (host store shared read-only via virtiofs), macOS via the
  Virtualization.framework/linux-builder pattern, Windows via NixOS-WSL. The VM
  lane is commodity (docker desktop, podman machine + gvproxy prove the recipe);
  track/tourvm is its first dogfood.
  (b) **No homegrown rootless imitation** ("totdat systemd rootless land zou ik
  het niet namaken"): the podman userns stack (setuid newuidmap/newgidmap,
  subuid ranges, pasta) is refused as parallel plumbing against D48(e).
  systemd's own unprivileged-sandboxing line is surfed, not rebuilt: interim
  stance is `--user` as fallback with its blanket "degraded" stamp upgraded to
  an honest per-capability graded banner driven by the existing D36 probes —
  every systemd release improves the tier without cix building anything.
  (c) Quadlet is acknowledged as prior art validating
  unit-generation-as-product; nothing of podman's runtime is adopted.
  (d) **(2026-07-31, Mathijs: "eens") The daemon route is the primary Linux
  rootless answer.** A thin privileged, socket-activated cix service on the
  nix-daemon pattern: the caller sends a store path plus a narrow, audited
  override surface (instance knowledge per D49(a)); the daemon compiles the
  unit itself — hardening non-negotiable, callers never supply raw properties.
  (The pure systemd+polkit route is refused for exactly that reason: polkit can
  scope unit *names*, but transient-unit creation lets callers set arbitrary
  properties (`User=root`) — the security boundary must lie on properties, and
  only cix's compiler knows the legal surface.) Escalation ceiling: a caller
  can at worst run a store item as DynamicUser under the full hardening
  profile — unlike docker's root-equivalent socket group. Explicit non-goals:
  no supervision (systemd's job), no building (nix-daemon's), no storage (the
  store's); the D48(d) identity registry lives in the daemon. `cix machine`
  becomes a *transport* for the same socket protocol (docker-context UX for
  free); vmspawn-vs-QEMU is decided evidence-gated in machine's own track.
  `--user` demotes to fallback with no standalone investment: the D36-probe
  graded banner rides whichever track next touches capabilities.rs.
  Sequencing: daemon design after the current corpus wave and before netns
  realization (D49) — netns and identities want the daemon as their home;
  machine after.

- ✅ D62 (2026-07-31) — **family tags: declared names, external tags, three honest
  layers** (dialogue round with Mathijs; prior-art scan: docker/compose/buildx-bake/
  OCI, nix flakes, cargo workspaces, maven, Go modules, npm scopes, skopeo. The
  winning shape is Go modules': *name in the source, version from outside* — with
  skopeo's amendment that the name must not be baked into the bytes).
  (a) **Three layers, spelled out.** (1) *Identity* = the store path:
  anonymous, content-addressed, never changed by naming (the skopeo invariant —
  promotion/retag is always a metadata operation on unchanged bytes). (2)
  *Declared identity* = names in the Cixfile: `SERVICE <name>` block names are
  the real member names (shipping labels — builders keep local binder names:
  workshop names local, dock names global). **Amendment (same day, Mathijs's
  YAGNI call): there is NO `NAMESPACE` directive.** The not-baked rule below
  had already hollowed it out to a mere default for the `-t` sugar — a claim
  binding nothing, with the `--namespace` flag existing anyway — and the
  human-facing "what is this called" need is D54 META territory when that
  builds (where a family name would also sit wrong mechanically: META fields
  are per-artifact-block and baked/hash-covered, a family name is per-file and
  deliberately unbaked). The operational family name is publisher knowledge
  and comes exclusively from `--namespace` at tag/publish time; with it gone
  from the file, Go's fork pain vanishes entirely rather than being mitigated.
  Nothing about naming is baked into the built item (no manifest field, no
  store bytes — else fork pain at byte level and every promotion becomes a
  rebuild). (3) *Naming* = index operations: tag/
  publish/promote are D45 table moves; `cix build -t` is sugar into this layer,
  never a build-layer act — the "naming belongs where?" impedance dissolves.
  (b) **`cix build .` (no `-t`) emits ONLY the member map as JSON**
  (`{"member": "/nix/store/…"}` — always the same shape, also for one member)
  and tags nothing: pure layer 1. **`cix build .#member` materializes just that
  member's backward DAG slice** (D47 backward references; D57 memoization makes
  the next member's build cheap) and prints the **bare store path** (scripting:
  `cix run $(cix build .#my-app)`). Grammar rule per layer: **`#` = build-side
  member selection** (nix muscle memory, local context) — no conflict with D41,
  which killed the selector *within* an item; **`/` = index-side member
  naming** (registry muscle memory): refs are `[host/]family/member:tag`,
  host recognized docker-style (first segment with a dot/port), family and
  member single segments.
  (c) **`-t <tag>` is tag-only, repeatable, and family-wide.** Multiple `-t`s
  publish atomically (D45 already promised atomic multi-tag; the semver cascade
  `-t v3 -t v3.2 -t v3.1.2` is three `-t`s — cascade *computation* stays
  tooling-on-top per D46's monorepo line). **Selector and `-t` exclude each
  other**: a tag names a coherent family at a version (Go lesson: you don't tag
  half a module); partial stamps would demand merge semantics on tag tables.
  Whole-family tagging is cheap anyway — unchanged members come from the memo.
  `--namespace <name>` supplies the family name (explicit CLI, so no ambient
  sin): **required when tagging a multi-artifact file** (bare sibling names
  must never leak into a global namespace — the D32 "AS is REQUIRED" taste);
  optional for a single-artifact file, whose member name is then the family.
  Optionally host-qualified (`--namespace cix.my-org.com/my-app`, go-module
  style), **schemeless** (scheme is transport, not identity; skopeo separates
  them too); a claim, not a grant — publish checks D45 name-level auth. Old
  spellings die with migration-grade errors: `-t name:tag`
  ("names moved into the Cixfile"), bare `-t v1`-multi-regime (bare block
  names in the index), dirname-derived anything (compose's ambient-identity
  sin — the dir name may appear in an error *suggestion*, never as silent
  identity).
  (d) **No implicit `:latest`, anywhere.** A ref without `:tag` is a parse
  error everywhere (docker's `:latest` is a silent default in tag position —
  the D12 sin family; refused on build, run, and pull). Moving pointers are
  ordinary explicitly-managed tags (`-t latest`, `-t stable` — no cix
  semantics), and the D45 history chain makes them auditable (when did
  `latest` move, to what) — something docker's `:latest` cannot answer.
  (e) **Index form: a family is ONE D45 name whose tag table maps
  `tag → {member → {storePath, narHash, meta}}`** — atomic whole-family
  publish is the existing CAS pointer move, auth stays name-level = family-
  level, resolvers stay dumb readers (member lookup is a map key after
  resolution), single-artifact is a one-member map (no structural special
  case). **Increment plan**: round one lands language+CLI with family/member
  as plain slashed names in the existing per-name tables (resolver-untouched;
  multi-member publish interim non-atomic, honestly N moves); the true family
  tables + atomic multi-member publish ride the D46 parametric-publish work.

- ✅ D64 (2026-07-31) — **the implicit self-bin: your runtime toolset IS your own
  `bin/`** (Mathijs, spotting the corpus's pre-D58 `PATH bin` idiom: a service
  putting its OWN bin tree on the runtime PATH is a *self-import* that can
  always be implicit — and that settles "compose a PATH from other dirs" vs
  "assemble your own bin/ tree" in favor of the latter).
  (a) SERVICE and APP get an implicit runtime `PATH=<item>/bin` default. An
  explicit `ENV PATH=…` REPLACES the default entirely — no merge magic
  (D58's YAGNI-return clause stays the escape for genuine multi-dir needs).
  (b) Bare `EXEC <name>` (and SETUP) resolves at build time against the
  *effective* PATH: the item's own `bin/` by default, or the declared
  `ENV PATH=…` when present (the D31 build-time resolution mechanics under
  the (a) replacement rule — e.g. `ENV PATH=${pkgs.redis}/bin` +
  `EXEC redis-server` resolves into the package); not found = clear error
  listing the searched entries. `EXEC ${pkgs.x}/bin/x` stays valid; the
  relative `EXEC bin/x` form died with D66.
  (c) `cix exec`/`debug` shells inherit the same PATH — D31's toolbox
  rationale collapses to "PATH = the item's /bin".
  (d) Minimal-magic audit: the default references nothing outside the artifact
  — self-referential and content-addressed, the D47-context-sugar class
  (ambient in spelling, never in content). External tools enter the toolset as
  visible `LINK ${pkgs.x}/bin/<tool> bin/<tool>` lines: the runtime toolset is
  enumerated in the tree, stronger provenance than D31's PATH lists ever had.
  Nothing beyond your own tree is ever implicit.

- ✅ D63 (2026-07-31) — **two acts, not two places: the anonymous loop and the
  naming act; the GC contract completed with runtime roots** (Mathijs's
  two-mode docker observation, critically sharpened; verdict "ja, links in
  /run lijkt me prima").
  (a) The design keys on the *act*, not the location: the **anonymous loop**
  (bare build → JSON member map, selector → bare path, run by store path;
  nothing named, everything collectable) versus the **naming act** (`-t`/
  publish, D45 table moves). Not local-vs-CI: PR pipelines run the loop in CI;
  laptops publish. Docker's local junk-tags are a workaround for unusable
  image-ID handles, not a want — store paths are usable handles, so cix
  delivers the loop docker only approximates.
  (b) GC: D7/D35(b) already give tag=root, untag=unrooted=collected, no prune
  machinery. The gap this closes: **`cix run` registers a unit-lifetime GC
  root** — an indirect root `/run/cix/gcroots/<unit>.root → <item>` (user
  mode: the XDG_RUNTIME_DIR analogue) created at start, removed by an
  injected `ExecStopPost=` (D48e-transparent). /run is tmpfs, so reboot
  self-heals stale roots; a dangling auto-side link is pruned by nix itself.
  Why needed: nix's /proc-scan runtime roots protect only currently-mapped/
  open paths — the restart path (the loaded ExecStart= string, an un-run
  ExecStartPre, later-opened files) is unprotected; nix's own temproots are
  connection-scoped, the wrong shape for detached services. Compose is
  already safe (D30 profiles are roots).
  (c) Compose-dev without tags: parked, evidence-gated (docker's `build:`
  section is the prior art); the honest bridge is a throwaway `-t dev`,
  fully unpinned again by untag per D7.

- ✅ D65 (2026-07-31) — **FROM's three input kinds; universe-tags resolved**
  (closes the open universe-tags design; Mathijs: "die FROM semantiek lijkt
  me inderdaad prima zo").
  (a) A flakeref is a *fetch address* (closed scheme set: `github:`, `git+…`,
  `path:`, `tarball+…`, `.`); **no flake.nix is required** — bare repos are
  first-class trees. FROM binds three kinds: (1) *tree via flakeref* →
  source binder (D47, exists); (2) *package universe via flakeref* →
  namespace binder (D32, exists) — the tree is imported classically
  (`import <tree> { system }`, i.e. the tree's `default.nix`), so the
  requirement is evaluability as a package set, not flake.nix. Any tree with
  such an entry point qualifies (nixpkgs, or a company repo whose default.nix
  returns nixpkgs+overlay); the mirror edge is documented, not a bug: a
  flake-ONLY repo (flake.nix, no default.nix) is not usable as a universe
  today — the known small extension (getFlake → `legacyPackages.<system>`
  fallback) is evidence-gated; (3) **NEW: cix item via index ref → artifact
  binder**: `FROM cix.my-org.com/acme/web-vault:v3 AS webvault` — resolved
  via the D45 index (pull if absent), narHash-verified, lock-pinned
  (`ref → {storePath, narHash}`; the ref may move, `--update-lock` moves it
  deliberately), usable as a COPY/LINK source. Docker's cross-image
  `COPY --from=<image>` made honest; corpus forcing example: vaultwarden's
  prebuilt web-vault.
  (b) Disambiguation via D62's own rules: a known flakeref scheme ⇒ flakeref;
  otherwise the token must parse as an index ref with an explicit `:tag`;
  otherwise an error naming both grammars. Local unqualified refs are allowed
  (the lock pins them); qualified refs give from-anywhere reproducibility.
  (c) **An index ref never binds a namespace** — universes stay flakeref-only
  (name provenance). Universe-tags land dissolved: the eval side IS the
  flakeref (nothing new); the prebuilt-prelude side is *substitution keyed on
  the lock pin* — availability plumbing, invisible to grammar; short
  spellings, if ever wanted, via D32's cix-owned constant table. The earlier
  pre-declared-IMPORTs inheritance idea is dead.
  (d) `IMPORT` of a cix item inside builders: deferred, evidence-gated.

- ✅ D66 (2026-07-31) — **artifact destinations are absolute: you declare places
  in your runtime world** (Mathijs, after the /srv/www-mount question exposed
  the COPY/LINK spelling split: "relatief dat toch een mount wordt is niet wat
  je zou verwachten — doe dan liever alles absoluut").
  (a) In SERVICE/APP blocks, every destination-like path is spelled ABSOLUTE:
  `COPY ${src}/index.html /srv/www/index.html`, `LINK <target> /etc/nginx/
  mime.types`, `FILE … /etc/app.conf`, `EXEC /bin/example`. The path names a
  place in *your item's world*, rooted at `/` = your item root — that it is
  concretely stored at `<item>/srv/www/…` and realized per top-level dir
  (bind-mount claims for `/etc`/`/srv`-class dirs, the D64 PATH for `/bin`)
  is realization, "almost an implementation detail". This unifies the block:
  STATEDIR/CACHEDIR/RUNDIR/PORT already spoke absolute runtime language —
  COPY was the odd one out. Writing outside your own artifact remains
  inexpressible: `/` IS yours, there is nothing else to name.
  (b) Relative destinations in artifact blocks DIE (migration-grade error
  naming the absolute spelling); `EXEC bin/x` follows (→ `/bin/x`); bare
  `EXEC x` (D64) is untouched. LINK linkpath becomes absolute-only (its
  both-spellings tolerance was the inconsistency that surfaced this).
  (c) **BUILDER destinations stay workdir-relative** (`COPY ${src}/ .`) — the
  workshop is a bench with a cwd, the dock ships to declared addresses. The
  workshop/dock doctrine now shows in path spelling too.
  (d) Docker muscle memory lands exactly right: Dockerfile `COPY x /app/x`
  translates 1:1, with the same honest meaning docker gives it — "inside your
  own root".
  **Addendum (same day, the "here" rule — Mathijs's principle, refined by its
  one counterexample): relative paths are coherent exactly where a *here*
  exists.** The workshop has a runtime here (the workdir: RUN, FETCH, builder
  COPY destinations, `$PWD` in builder ENV); the Cixfile text has an authoring
  here (its own directory — the D47 bare-relative COPY-source context sugar,
  legal in artifact blocks too, is relative to THAT here, not to any cwd).
  The dock, the index, and the runtime namespace have no here — hence
  absolute. Runtime-relative argv paths belong to the process's own cwd; the
  language stays out of them.

- ✅ D67 (2026-07-31) — **the strata and the distribution inversion** (a long
  dialogue round with Mathijs; his articulation, jointly stress-tested. The
  origin story frames it: composix arose from pulling one thread — "can
  nix+systemd replace docker" — and the products that emerged decompose along
  real seams).
  (a) **The strata.** (1a) *manifest/runner*: a directory plus cix-manifest
  declares "this can become a hardened systemd unit" — the schema is
  nix-independent (evidence: vm-dogfood hand-writes manifests onto runCommand
  outputs; D54's cix-parses-nobody's-manifest; the D20 capability thesis),
  with two honest nix anchors today: `cix run` takes store paths, and the
  *integrity* story (closure-as-only-true-manifest D32, D63 GC roots) is
  store-bound — detachable at the cost of the trust story, not of
  runnability. (1b) *compose*: composites over manifested items; nix-anchor
  is profile atomicity (D30), an implementation choice. (2) *the Cixfile*:
  the fix for nix's building story — builders and (future) pure items are
  stratum 2; **SERVICE/APP are the 2→1a adapters**, and the seam runs through
  the middle of those blocks (assembly directives vs contract directives) —
  Dockerfile's own shape, which the adoption bridge (D16) justifies.
  (3) *the index*: nix-native naming/distribution — D45 tables over store
  paths, manifest-agnostic by construction.
  (b) **The necessity chain: 2 ⇒ 3 ⇒ 1a.** A Cixfile build is store-native,
  NOT derivation-native (cix orchestrates builders with D57 keys, warm
  workspaces, and pinned network FETCH — none of which are expressible as
  pure derivations) — so recipients cannot re-derive, so distribution must be
  by artifact (3), and what arrives must run without evaluation — which is
  what (1a) is. Flake-interop (a flake depending on a Cixfile output through
  eval) is a deliberate NON-GOAL: the same choice docker made against distro
  packaging — own build world, shared artifact world (the store).
  (c) **The inversion: nix distributes recipes (bytes as an optimization);
  cix distributes artifacts (the recipe as optional provenance).** The flake
  model forces consumers to evaluate and to choose between world-duplication
  (follow the publisher's lock, hit their cache, carry their nixpkgs) and
  recompilation-with-skew (`follows` their own); the cix consumer path —
  resolve name → path → bytes → run — is evaluation-free end to end (dumb
  D45 resolvers, manifest-as-data). **Keystone: by-artifact distribution is
  what LICENSES the workshop's nondeterminism tolerance (D39.1)** — recipe
  distribution must be deterministic because the recipe is the object; we
  may tolerate nondeterminism because the bytes are the object, and
  determinism remains an opt-in (EXPECT-pinned inputs, deterministic dock,
  `--cold` as the audit verb) instead of an entry fee.
  (d) **Prior work honestly weighed**: the byte layer is solved upstream and
  we ride it (substituters/narinfo signing; `fetchClosure` is nix's own
  acknowledgment of artifact-first consumption; CA-derivations move identity
  bytes-ward). The NAME layer is the open lane: channels are coarse,
  FlakeHub versions recipes (semver over flakes, pin in the lock — they too
  keep the pin out of the version), Flox catalogs packages; none have
  mutable name→tag tables with a content-addressed history chain and
  name-level auth. The strongest demand evidence is the ecosystem's flight
  to OCI (dockerTools/nix2container/nix-snapshotter): people wrap store
  outputs in image tarballs purely to obtain name:tag+registry.
  (e) **Everything moves; the differentiators are pin quality and audit
  quality.** Docker's variant tags (`3.15-slim-bookworm`) move invisibly and
  its pin (digest) is second-class, so builds are unpinned in practice;
  nixpkgs channels move with git-grade audit; cix enforces first-class pins
  (locks everywhere, D32) and gives artifact tags git-grade audit (the D45
  chain). Corollary: the nixpkgs pin belongs in table META and receipts
  (hash-true per D54), never in the tag string — docker's variant-tag
  cross-product is the symptom of tags-as-only-metadata; variant tags remain
  available as mere convention for genuine multi-world publishers.
  Freshness/CVE republish = a deliberate, audited move of the SAME tag plus
  the D46 family bump.
  (f) **The world moves in plateaus and avalanches** (a package's path
  changes only when its transitive drv inputs change; stdenv/glibc bumps
  avalanche — the staging workflow exists for this). Content-addressing
  makes cix publications mirror that sparseness: on a pin bump only
  truly-changed members get new bytes, the family move stays one table op,
  and the chain shows no-op members honestly. Caveat inherited from
  input-addressed nix: path churn outruns byte churn (no early cutoff), and
  our items embed those paths; upstream CA-derivations (hash-modulo
  self-reference rewriting + signed realisations — themselves kin to our
  signed name→bytes tables) would lengthen our plateaus for free.
  (g) **Early vs late binding — the runner's second dividend.** Nix binds
  references at build time into bytes (self-sufficient anywhere; everything
  churns together). Cix binds the content layer at RUN time via namespace
  projection (D66): store references concentrate in the thin generated
  layer (manifest argv, LINK targets) while authored content speaks
  projected absolute paths — which is what makes stratum 1a separable,
  keeps bump deltas small, makes items CA-easy (no self-references), and
  preserves docker's relocatable-content property. The price, stated: a
  projection-capable runner must exist at runtime; the manifest is the
  binding record. **Isolation and relocation are the same mechanism** — the
  namespace adopted for hardening turned out to be the projection engine;
  principled constraints keep paying where they were never aimed.
  (h) **The honest price list**: visible-but-real nixpkgs skew between
  publishers (docker pays it blindly; our closures/locks/receipts at least
  show it); the CVE-republish duty moves to the publisher (softened by (f));
  and the trust ladder — reproduce it yourself / trust a signer / trust
  whoever fills the store — on which cix currently stands at rung three
  until D35 signing lands, with richer provenance than either neighbor
  (verifiable SOURCE claims, naming audit) and rung one available per item
  as an option. Upstreaming to nixpkgs and publishing to a cix index share
  the same ultimate requirement: you trust whoever fills the store.
  **Open product questions, deliberately unresolved**: whether stratum 1a is
  ever sold separately (manifest-on-a-tarball as a product) or stays an
  internal seam; and whether tool distribution (the gitsitter itch —
  `cix install` sugar vs pointing at `nix profile install <path>`) is index
  scope. Both need forcing examples, not forward design.

- ✅ D68 (2026-07-31) — **ITEM returns as a manifest-less pure store tree**
  (Mathijs's correction during the strata round: "voor een ITEM willen we
  toch juist [geen] manifest — een manifest is een systemd/cap-dec shaped
  ding"; supersedes D50's drop, whose evidence was zero consumers — the
  consumers exist now: D65 FROM-item binders and the stratum-2
  build-language use-case, forced concretely by the gitsitter/crane
  comparison).
  An ITEM block assembles a bare store tree with **NO cix-manifest.json** —
  the manifest is stratum-1a vocabulary (D67) and an item is not runnable.
  Allowed directives: COPY/FILE/LINK (pure assembly). Stratum-1a vocabulary
  (EXEC/SETUP/ENV/PORT/LISTENER/GRANT/role dirs/health) is a parse error
  naming the seam ("items are build products; SERVICE/APP declare runnable
  contracts"); `cix run` on a manifest-less item errors the same way. Items
  are `.#`-selectable, D65-consumable, and taggable/publishable — stratum 3
  is manifest-agnostic by construction.

- ✅ D69 (2026-08-01) — **FETCH pin stability** (the round forced by four
  exhibits across three ecosystems: dozzle's go-sumdb tiles — seven files,
  35,808 B, precisely diffed; parse-server's npm ci flapping on every run;
  dozzle's pnpm UI cache; projB's stale cargo pin caught by cold-audit's
  first-ever sweep. Root diagnosis: FETCH pins the whole workdir tree, but
  package-manager fetches emit *payload + incidental cache state*, and the
  incidental part is nondeterministic by design — docker never notices
  because docker pins nothing).
  (a) **Consumed-set keying** (Mathijs: "obviously correct, free win"): the
  automatic FETCH pin narrows from whole-tree to the paths downstream steps
  actually consume — D57's narrow read-keying extended to fetch outputs.
  Unread bytes cannot influence the output, so the pin still covers
  everything that matters. Accepted consequence, stated: the pin re-keys
  when later steps start reading MORE (first-use of a previously unread file
  is recorded loudly as a fresh pin) — honest, defensible. A declared
  `EXPECT` stays what it is: an author-level whole-tree integrity claim for
  those who want that strength.
  (b) **The `--update-lock` double-fetch instability probe** (Mathijs's
  mechanism, discovery-as-warning not discovery-as-surgery): when a pin is
  set or moved, cix runs the fetch twice, diffs, reports loudly which files
  are volatile (names + sizes), and records the volatile set in the lock as
  fact. This answers "why is this fetch unstable" *structurally* at the
  moment it matters, and feeds both (a)'s verification and (c)'s authoring.
  (c) **Volatile bytes inside consumed files** are authoring territory:
  normalize in the fetch command itself (cache outside the pinned root,
  prune volatile metadata) — per-ecosystem recipes taught by docs/migrate.md,
  zero cix mechanism; the probe names the targets.
  (d) **Refused**: a declarative `STABLE FETCH` (auto-rm of a two-run
  intersection falls between the stools: probabilistic where consumed-keying
  is deterministic, unsafe exactly where normalization is needed); and
  functions in `${…}` (reaffirming the D32 line — `${…}` stays a pure,
  provenance-auditable name lookup; composition lives in nix-land).
  (e) **`--cold` never refetches — cold rebuilds are offline by construction**
  (resolving the asymmetry the diagnosis exposed: in-builder FETCH steps
  re-ran under --cold while top-level FETCH did not). Unified semantics:
  `--cold` replays RUN steps from empty workspaces but reuses pinned FETCH
  outputs of both kinds; fetch re-execution belongs exclusively to
  `--update-lock` and the (b) probe — the moments you deliberately ask the
  world. Documented boundary: --cold proves builder reproducibility; fetch
  trust IS the pin. Companion fix in the same round: memo/chain keys gain a
  cix codegen fingerprint (the cross-version workspace-pollution bug: a
  stale checkout getting memo hits from bytes built by newer cix — caught
  via tour narHash drift).

- ✅ D70 (2026-08-01) — **overlay universes: the escape hatch for package
  composition** (the wallos/`php.withExtensions` forcing example; a dialogue
  arc worth keeping honest: sidecar-package → explicit `USING` injection →
  Mathijs's "USING is a function call in a jacket" and "why not a file that
  IS nixpkgs-except-my-php" → this).
  (a) **`FROM <flakeref> OVERLAY <./file.nix>… AS <name>`**: the base
  universe evaluated with nixpkgs' own overlay mechanism —
  `import <tree> { system; overlays = [ (import ./file.nix) … ]; }`. The
  overlay file is the bare nix idiom (`final: prev: { php =
  prev.php83.withExtensions …; }` — three lines); repeatable, order =
  overlay order. Implementation is one argument on the existing classic
  import; **getFlake stays evidence-gated** (this removes its forcing
  example). Computation lives in the .nix file, the Cixfile only wires a
  path — the same boundary that refused functions-in-`${…}` (D69d) and
  killed `USING`.
  (b) **Requirements, checked with clear errors**: the base must accept an
  `overlays` argument (functionArgs-checked; else "wrap the base or use a
  full universe tree"); the overlay file must be a `final: prev:` function
  to attrset. Semantics are the real nixpkgs fixpoint — an overlay can
  override deep (`openssl`) and the whole `${pkgs.*}` world follows; power
  and cost both standard and visible.
  (c) **Keying/lock**: universe identity in chain keys = (base pin, ordered
  overlay file hashes); overlay files are context content; one lock —
  `--update-lock` moves the base pin, overlay edits are ordinary source
  edits. Overlays cannot reference Cixfile binders (pure final/prev —
  source-dependent building is builder territory). Multiple universes side
  by side stay legal (name provenance) with the documented hygiene note
  that this deliberately reopens world-skew inside one item. Eval-time
  impurity in .nix files (unpinned fetchTarball) is the author's own,
  identical to full universe trees — documented, not newly introduced.
  (d) Full universe trees (a repo's default.nix that IS
  nixpkgs-plus-overlay) remain the general mechanism per D65 for org-owned
  worlds; `OVERLAY` is the project-local pretty form. The composed-ITEM
  route (D68+D65) stays the org-wide distribution form.

- ✅ D71 (2026-08-01) — **the underlay: builder runs always start on their own
  last end-state** (Mathijs's framing, closing the warm-edit gap the
  nixcompare measurement exposed: both we and crane discarded own-step
  increments, so our warm edit was a full recompile plus overhead. The
  doctrine already promised this model — D47(e)'s workshop is "persistent,
  disposable, may be messy" — the implementation was stricter than the
  prose; this restores the prose, now that the safety nets exist).
  (a) **Underlay-always**: a builder re-run starts from the last end-state of
  the SAME builder (same project workspace, same builder name) as its lower
  layer; what this run writes is the upper. No opt-in keyword — it IS the
  workshop semantics. `rm -rf` workspace = drop underlay = cold: "deleting a
  workspace is always correct" stays literally true.
  (b) **`--cold` = without underlay** — the clean semantics stay definable
  and are the audit verb (offline per D69(e)).
  (c) **The ghost-file hazard, named**: warm results are path-dependent —
  build(A→B) may differ from build(B) (deleted sources/deps surviving in
  underlay state). Accepted under D39.1 because the correctness boundary is
  the dock, guarded by consumed-output records + the STANDING cold audit
  (D47e made real) + the codegen fingerprint (D69e). BuildKit cache-mounts
  ship exactly this hazard with no audit verb; we ship it with one.
  (d) Workspace growth is LRU policy, never correctness. Cross-builder or
  cross-project underlay reuse does not exist.
  (e) Living receipts: migrate.md's `RUN --mount=type=cache` row becomes
  fully true (previously subtly overclaimed vs BuildKit), and
  docs/nix-build.md re-measures the warm-edit column (expectation: real
  cargo increments, 2–5s where crane does 16.5s).
  **Retreat path, recorded up front (Mathijs): if the aggressive
  underlaying ever chafes** (ghost-file wrongness showing up faster than the
  sampled cold audit catches it, or path-dependence confusing users),
  **CACHE returns** — the D48(a)-era declared exception, making the warm
  surface opt-in per path instead of whole-workspace-always. The dial goes
  underlay-always → CACHE-declared → prefix-only; we start at the warm end
  deliberately, with the audit verb as the regulator.

- ✅ D72 (2026-08-01) — **alpha compat posture: the manifest version is 0**
  (Mathijs: "we zijn alpha — noem de alpha spec v0; alle manifests en
  Cixfiles leven in deze repo, voor nu is het ons feestje"). Pre-1.0 there
  is exactly ONE manifest version and it is `cixManifest: 0`: its schema
  changes freely with the language — no bumps, no ranges, no compat matrix.
  The v1–v5 numbering and the entire back-compat validation in
  cix-run/spec.rs die; any non-zero (or unknown-shaped) manifest gets a
  hard, friendly "rebuild with the current cix" error. All in-repo manifests
  and fixtures sweep to 0 (vm-dogfood's hand-written v2 manifests
  included). D15's version-gating regime is SUSPENDED for the alpha and
  returns at 1.0, when `cixManifest: 1` becomes the first versioned,
  compatibility-bearing schema — a real cost-benefit decision then, not an
  accreting alpha tax now.

- ✅ D73 (2026-08-01) — **the decomposition round; complexity is actively
  managed** (Mathijs: "probeer het complexiteitsmonster te allen tijde onder
  controle te houden" — recorded as a house principle alongside D48(e):
  measure periodically, decompose along the strata, thin the hotspots
  before they calcify. Baseline this day: ~19.6k SLOC Rust; hotspots
  parser.rs 2.6k, build_chain.rs 1.9k, index/lib.rs 1.6k, spec.rs 1.1k).
  (a) **Crate split: `cix-build` leaves `cix-cixfile`** — the workshop
  engine (chain, keying, workspaces, sandbox) is stratum-2 machinery, the
  parser/codegen are language; the crate graph should mirror the D67 seams.
  (b) **cix-index splits into modules** (refs, tags, roots, serve, pull)
  with a thin lib.rs.
  (c) **spec.rs collapses to the single v0 schema** (D72).
  (d) Parser diet per the analysis report (adopted: modules + tests out of
  the file, declarative migration table, validator consolidation; generated
  metadata explicitly NOT warranted for a 22-keyword language).
  Sequencing: after pinkeys merges; the underlay (D71) then lands in the
  new cix-build crate — a clean home instead of a bigger pile.
  **Addendum (2026-08-01, Mathijs): user-facing diagnostics never cite
  D-numbers** — design-journal references are internals; messages point at
  stable doc anchors (docs/cixfile.md sections) instead, with D-numbers
  surviving only as code comments beside the message. Rides track/crunchy
  for the sweep; the declarative migration table keeps the D-number as an
  internal field.

- ✅ D74 (2026-08-01) — **`cix fmt`** (prior-art round with Mathijs; the
  post-gofmt consensus — cargo/black/terraform/zig/deno — adopted, minus one
  flag). `cix fmt [PATH…]`: default `.`, recursive discovery of `Cixfile`s,
  .gitignore-respecting, apply-in-place, parse-gated (unparseable file ⇒
  the ordinary parse error, nothing written), idempotent.
  **`--check` = verify AND explain**: no writes, exit 1, prints the per-file
  unified diff (Mathijs's merge of check+diff: at Cixfile scale the diff IS
  the explanation, and a name-only check forces a second local run; the
  big-repo noise argument does not apply). `-` = stdin→stdout for editors.
  No other flags. Canon v1, deliberately minimal: block bodies indented two
  spaces, blank line between blocks, prelude FROMs unindented at top, single
  spaces between tokens and around `=`, trailing whitespace and CRLF
  normalized; comments preserved verbatim (D53 — this forces the
  trivia-preserving printer, which is the real work), heredoc bodies
  untouched. Alignment/sorting = v2, evidence-gated. `cix fmt --check`
  joins the standard gate.

## Non-goals (for now)

Hosting nars (D6, modulo O2) · multi-host orchestration · per-service netns · build-on-pull ·
non-systemd runtimes.
