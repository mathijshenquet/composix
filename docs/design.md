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
| `/{name}` | tag table (tag, systems, closure size, narHash, age), pull snippet, spec summary if the store path carries `cix-spec.json` | `{"tags": {"latest": <entry>, …}}` |
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

### Spec v2 (decided 2026-07-28, after the nginx/postgres dogfood round)

Two principles first, more important than any field:

- **D20a — no raw systemd passthrough, ever.** The spec declares capabilities in *app
  semantics* ("I JIT", "I listen on 80", "I need a socket dir"); the generator maps to
  mechanism. Raw overrides are operator territory → compose, never the spec.
- **D20b — the boundary**: spec = what the supervisor must know (lifecycle, resources,
  capabilities); item = app quirks (nss_wrapper, path plumbing); compose = operator decisions
  (overrides, config content, scaling).

v2 fields/changes (`"cixSpec": 2`; runner accepts 1 and 2; new fields rejected under 1):

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
- **D22 — the stable item mount `/item`.** Every system-mode unit gets its store item bound
  read-only at `/item` (plus `CIX_ITEM=/item` in the environment). This restores docker's
  stable-absolute-paths property: config files and scripts reference `/item/…` and stay
  VERBATIM — no build-time templating in file contents, ever. Cross-package references are
  pulled into the item via links (Cixfile `LINK`), so `${pkg}` interpolation is confined to
  directive arguments. In degraded `--user` mode (no binds) `CIX_ITEM` is the real store path
  and `/item`-dependent items warn loudly.

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

Service directives (compile to cix-spec v2): `SERVICE <name>`, `EXEC`, `SETUP`,
`ENV NAME [= default] [required] [secret]` (docker-compatible: `ENV FOO = bar` behaves like
docker's), `PORT name = $VAR` (env form) / `PORT name = 8080` (value form), `STATE` `CACHE`
`LOGS` `CONFIG` `RUNDIR` (role dirs, D11-narrowed paths), `JIT`.

Interpolation rule: `${…}` (build-time) lives in directive arguments and in `FILE`/`SCRIPT`
heredoc bodies (`$${…}` escapes to a literal); `COPY`'d files are always verbatim; `$VAR`
(runtime env) only in EXEC/SETUP. `/item` paths (D22) remove the *need* for interpolating
file contents — heredocs merely retain the option. There is no RUN, deliberately.

Determinism: `cix build [dir] [-t ref]` compiles Cixfile → nix expr → store item. nixpkgs is
pinned in `Cixfile.lock` (rev + narHash; created on first build, `--update-lock` to roll).

## Part 4 addendum — beyond v1 (design pending)

- Positioning per D16: not essential to baseline value, but where the non-nix audience is won.
  Sequenced last; `composix.lib.withSpec` (nix helper) lands much earlier as the nix-native rung.
- Two halves: BUILD (project → store item) and SERVICE (→ spec). SERVICE half is settled by the
  part-2 schema; blocks compile to `cix-spec.json`.
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

## Non-goals (for now)

Hosting nars (D6, modulo O2) · multi-host orchestration · per-service netns · build-on-pull ·
non-systemd runtimes.
