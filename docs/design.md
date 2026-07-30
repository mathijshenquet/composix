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
   BoundingSet). No new field: the declaration is the grant. (Socket activation: compose era.)
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
`FILE <dst> <<EOF` / `SCRIPT <dst> <<EOF` (inline, `${…}`-interpolated; SCRIPT adds shebang +
exec bit), `LINK <dst> <target>`.

Service directives (compile to spec v2): `SERVICE <name>`, `EXEC`, `SETUP`,
`ENV NAME [= default] [required] [secret]` (docker-compatible: `ENV FOO = bar` behaves like
docker's), `PORT name = $VAR` (env form) / `PORT name = 8080` (value form), `STATE` `CACHE`
`LOGS` `CONFIG` `RUNDIR` (role dirs, D11-narrowed paths), `JIT`.

Interpolation rule: `${…}` (build-time) lives in directive arguments and in `FILE`/`SCRIPT`
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
  `docs/compose-formats.md` stands as the encoding archive: TOML (its recommendation) and the
  Cixfile-DSL become *candidate sugar encodings*, evidence-gated on people actually
  hand-writing composites at scale. Symmetry worth naming: Cixfile : item :: your-generator :
  composite — human languages at the edges, JSON contracts throughout.

## Building now (decided 2026-07-29, Mathijs: "bouw maar naar eigen inzicht")

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
  docs/compose-tree.md.
- ✅ D43 (2026-07-30) — **pod-ness is a scoped property, not a stratum.** `network: "pod"`
  on any composite claims one netns for its subtree; **nearest-pod-ancestor** decides
  each service's namespace (pod-in-pod legal; embedding never strips a sealed artifact's
  declared boundary). Crossing a boundary requires declaration: `publish` climbs one
  boundary at a time (child publish → parent scope, where it's `bind`-ed or re-published),
  `edges` (D25 fd tier) cross netns for free. Absence of pod-ness anywhere = today's
  host networking (rawdog = absence of a property; the `network: host` escape flag
  dies). Per-service **`egress: true`** (final spelling per D48(b); D20-side app
  semantics) declares outward initiation; absence = loopback-only view, zero network
  machinery for pure composites. Amends D23's fixed boundary; compose-netns.md remains
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
  docs/compose-netns.md; closes that paper's open decisions):
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
  selector grammar, zero Norway problem. What META itself accepts:
  - `META <field> "literal"`;
  - `META <field> <reference>` — for inline fields the trimmed FILE CONTENTS become
    the value (`META description ${build}/output/description`); for document
    fields (readme/icon/license-file) the file itself is taken into the item;
  - `META <field> ${pkgs.x}#attr.path` — attrset selection via the eval that
    already resolves package refs (nix-attrpath syntax incl. quoted segments);
    NOT file parsing — the dissolves-class keeps its one-liner;
  - `META source <binder>` — provenance designation only.
  Honest cost: a pure-assembly artifact wanting a field from its own manifest
  opens a mini-builder for the extraction, or writes the literal. `META source <binder>` keeps ONLY the
  provenance-designation role: a remote flakeref binder yields url+rev+narHash (the
  flakeref IS the url); a `.` binder yields **narHash-only** — a local working copy
  has no public URL, and claiming one from `.git/config` would be ambient host
  state (the D12 class) that lies on dirty trees. Steering side-effect: publishable
  provenance means building from a remote FROM — the reproducible-release flow.
  So: **IN** (priority on conflict): (1) explicit Cixfile META
  fields (literal or selector), (2) `META source <binder>` provenance, (3) derived WITH
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

## Non-goals (for now)

Hosting nars (D6, modulo O2) · multi-host orchestration · per-service netns · build-on-pull ·
non-systemd runtimes.
