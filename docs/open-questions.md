# Open questions

Rewritten 2026-08-04 (Mathijs: resolved items out, every entry
self-contained, larger items promoted to CIP drafts). Rules of this
document: an entry either carries enough context to act on without
opening another file, or it does not belong here. Resolved work lives
in `.dev/LOG.md` and the CIP changelogs, not in this file. Design-sized
questions live in `cips/draft/` — this file only points at them.

## In flight (agent tracks running or queued — no action needed)

- **EXPECT accepted without warm validation** — a wrong or copy-pasted
  `EXPECT` hash builds green as long as the fetch memo-hits; the
  mismatch only fires on a real refetch, possibly weeks later. Found
  because luna pasted one traefik fetch's hash onto the other and
  everything stayed green. Fix (in `track/buildfixes`): compare the
  declared EXPECT against the recorded lock pin at plan time and fail
  with both values named; also root-cause why the lock recorded the
  same pin hash for two different fetches.
- **A build error with no context** — the directus build dies with
  literally `Error: Not a directory`: no path, no build step named. You
  cannot debug what you cannot locate; every I/O error cix raises
  should say what it was doing to which path. In `track/buildfixes`,
  which also root-causes the underlying failure (it is the last thing
  between directus and a green build now that CIP-95 fixed its loader
  problem).
- **Editing a Cixfile can wedge its warm workspace** — after changing
  COPY lines, a re-run against the existing warm builder workspace can
  fail with `destination "go.mod" is already populated` instead of
  reconciling or invalidating the workspace. A worker "solved" this by
  contorting the Cixfile, which is exactly backwards — the workspace is
  disposable by design, the file is not. In `track/buildfixes`.

## Promoted to CIP drafts (read there; each is self-contained)

- [granular-degradation](../cips/draft/granular-degradation.md) — one
  rejected systemd directive currently drops a whole hardening set
  (this is why the tour broke on GitHub CI).
- [lock-scale](../cips/draft/lock-scale.md) — node-ecosystem builds
  grow Cixfile.lock by 100k–400k lines; compress observations without
  losing per-dependency reviewability.
- [volatile-fetch](../cips/draft/volatile-fetch.md) — some fetched
  content mutates every refetch (API JSON download counters, package-
  manager caches), making pins time bombs; teach + lint + normalize.
- [optional-env](../cips/draft/optional-env.md) — no way to declare an
  ENV that is optional with no default; a worker invented a
  `__cix_unset__` sentinel, which is the language failing.
- [artifact-root-collision](../cips/draft/artifact-root-collision.md) —
  role dirs under the application tree can collide with the artifact's
  own mount (wallos was forced from `/var/www` to `/app`).
- [tmp-relocate](../cips/draft/tmp-relocate.md) — cix probes/cold
  audits leave gigabyte node-trees in /tmp; they exhausted this host's
  tmpfs inode cap and froze every tool on the machine.

Also already in your inbox from earlier rounds: none — the draft inbox
otherwise contains only these six.

## Awaiting Mathijs — docker.md ledger dispositions (batch-blessable)

Each of these is a Docker feature whose ledger row in `docs/docker.md`
still carries a ❓ (undecided) or ⏳ (recorded-but-unbuilt) marker. The
proposed verdicts, with context:

- **`docker cp`** (copy files in/out of a running container) →
  propose ❌. A cix service's writable state lives in role directories,
  which are ordinary host paths you can reach with `cp`; `cix inspect`
  prints where they are. There is nothing to tunnel through a daemon.
- **`--name` (stable container handle)** → propose ⏳. Compose members
  already have stable names; a `cix run --name` for one-off runs is
  mechanical sugar on the existing run path — build it when someone
  actually asks.
- **`STOPSIGNAL` and stop timeouts** (which signal stops the process,
  how long to wait before SIGKILL) → propose ⏳, small mechanical
  track: these map one-to-one onto systemd's `KillSignal=` and
  `TimeoutStopSec=` unit fields; no design needed. Two corpus cases
  (adminer, nginx) currently note the upstream signal contract as a
  gap, so it has a real consumer.
- **Namespace sharing (`--ipc`, `--pid`, `--uts` between containers)**
  → propose: pods are the answer, standalone flags stay ❌. The
  compose tree already realizes shared network namespaces for a
  subtree ("pods", built in CIP-86); sharing IPC/PID follows the same
  pod mechanism (systemd's `JoinsNamespaceOf=`) if a case ever needs
  it, and per-pair ad-hoc sharing flags are refused.
- **Restart policy knobs** (`--restart=always` etc.) → propose:
  covered, tuning later. LIVENESS (CIP-79) is the deliberate restart
  opt-in with a fixed bounded policy; making the interval/burst
  configurable is compose-mechanical follow-up when a case demands it.
- **`docker init`** (generates a starter Dockerfile) → propose ⏳:
  the migrate teaching prompt is our generator today; a `cix init`
  skeleton belongs to a later tooling era.
- **`ENV NAME=value` (Docker's no-spaces form)** → propose: keep one
  grammar, improve the error. Today it is a parse failure; the
  diagnostic should say "write `ENV NAME = value`" so Docker muscle
  memory gets a teaching error instead of a second accepted spelling.
- **Docker Offload** (paid remote builds) → propose ❌: nix remote
  builders are the native answer.
- **AppArmor/SELinux label options** → propose: out of manifest scope —
  that is host security policy; revisit only if a real SELinux-host
  user appears.
- **Docker Desktop "Enhanced Container Isolation"** → propose ❌:
  a desktop-product threat model we do not share.
- **Authorization plugins** (pluggable allow/deny on engine calls) →
  propose: no plugin interface ever; policy questions return in the
  server/reconciler era (the decided deferral of a long-running cix
  daemon — docs/design.md D9).
- **Engine API / SDKs** → same reconciler-era deferral as above.
- **Remote contexts / `DOCKER_HOST`** → propose: ssh is the transport;
  any sugar is ⏳.
- **`docker mcp`** → propose ❌: unrelated to the runtime thesis.
- **Linux capabilities beyond NET_BIND_SERVICE** → propose: grow the
  `CLAIM` vocabulary case-by-case as dogfood demands (CIP-78 added
  gpu/device this way); never a raw `--cap-add` passthrough.
- **`ARG`/build args re-marking** → propose 🔁 (re-marked as "different
  mechanism"): cix deliberately has no CLI build-arg channel — the
  Cixfile text itself is the parameter surface (generate or edit it),
  and parametric composes cover deploy-time variation. The gitea
  version-stamp pattern gets documented as an idiom, not a mechanism.

## Era-parked (deliberate deferrals — context, then silence)

- **Publish era** — everything about *sharing* indexes publicly: push
  to a remote index, auth/login, signing entries, mirrors and
  pull-through caches, a hub with search/webhooks, SBOM/attestation
  exchange. Parked as one coherent future era (recorded as decisions
  D17/D35 in docs/design.md); today's serve/pull covers the local and
  trusted-network story.
- **Named-network era** — first-class network objects: service DNS
  names, `talks-to` allow-lists between services, cross-composite and
  multi-host networking, per-service IP/DNS/hostname options. Parked
  as decisions D26/D27. What IS built (CIP-86): per-subtree shared
  network namespaces (pods), port publish, egress with persistent
  addressing, and closed-root resolver projection.
- **Compose v1+** — replicas/scale, resource limits, reusable config
  objects, live update of a running composite. Parked in the compose
  scope decision (D30): v0 is deliberately lean and each omission is
  recorded in the docker.md ledger.
