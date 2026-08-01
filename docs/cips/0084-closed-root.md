# Closed root: runtime hermeticity, mandatory

Status: **CIP-84, adopted 2026-08-01** (Mathijs: "de rest kan als
proposed", after his whole-store argument won §4.1 — see §5). Mandatory
(no rawdog/opt-out dial), nix-coupling embraced (§3, "strands").

## 1. The problem

Service units today run under `ProtectSystem=strict` — a *write*
shield, not a *visibility* boundary. The entire host filesystem is
readable: /etc, /opt, and — most treacherously — the complete
/nix/store. A pack can read host config or hardcode a store path that
happens to exist on the author's machine (from an unrelated build) and
it works — until it deploys on a host where that path was never
realized. That is works-on-my-machine, structurally; it silently
un-enforces the core thesis D32 ("the closure is the only non-lying
manifest": only true if the closure is the only visible world) and it
leans on NixOS host magic (/bin/sh, /usr/bin/env — the CIP-80 dialogue
found `START /bin/sh` works by host luck) without enforcing anything
ourselves. Meanwhile our packs are already *more* hermetic than the
runtime demands — the postgres pack ships nss_wrapper.

## 2. Prior work

**Docker is already a closed root.** Nothing outside the image exists
unless explicitly mounted — works-on-my-machine died by construction,
and nobody calls docker unusable for it. The price was the base-image
tax: every image ships a complete userland (hundreds of MB, CVE
treadmill). Docker's answers to the hard edges split three ways:
*ship-it-in-the-image* (passwd/group from the base image; CA trust
store; tzdata), *engine-injects* (/etc/resolv.conf — managed bind
mount, filtered host resolv or the embedded 127.0.0.11 DNS proxy —
plus /etc/hosts, /etc/hostname), and *unsolved* (volume uid hell;
arbitrary-uid getpwuid failures → the nss_wrapper cottage industry).

**Kubernetes** mirrors this: kubelet injects DNS config, discourages
hostPath, images ship the rest.

**Composix's own builder** (bwrap, D57/D58): RUN steps already execute
in a world assembled from exactly the union of IMPORTs, plus one
enforced special case — `/usr/bin/env → /bin/env`, a symlink into the
union that dangles unless something declared ships env, with a
diagnostic teaching `IMPORT ${pkgs.coreutils}`. The runtime question is
"why doesn't the service get the builder's honesty?"

**systemd** has the mechanism natively: `RootDirectory=` (empty root) +
`MountAPIVFS=yes` (/proc,/sys,/dev) + `BindReadOnlyPaths=` whitelist +
the existing `PrivateTmp`/`PrivateDevices`/dir-classes. NixOS's
hardened module tradition proves services run fine in such worlds.

## 3. Recommendation

**Every service unit gets a closed root — mandatory, no opt-out.**
The generated unit uses `RootDirectory=` on an empty per-unit root +
`MountAPIVFS=yes`, then binds in exactly:

- the store closure of the item (computed at generation time from the
  lock-pinned item's references — `nix path-info -r` equivalent),
  read-only, per path (see §4.1 for the whole-store alternative);
- the item's D22 projections at their declared absolute paths;
- role dirs per the dirs CIP (state/cache/logs/run), claims-derived
  extras (shared surfaces, `CLAIM mount` paths, devices);
- `/usr/bin/env` as the D58 runtime analogue: a symlink resolving into
  the closure, dangling unless coreutils (or another env provider) is
  in it — same diagnostic. **No /bin/sh, ever**: the shell is a named
  dependency (`START ${pkgs.bash}/bin/sh -c '…'`) or absent. This
  closes the CIP-80 shell-form question: the canonical spelling is the
  only spelling, enforced identically on every host.

**No rawdog dial** (Mathijs's call — the sealed box wins): host-coupled
needs are spelled as claims, loudly, per D49(a) — `CLAIM egress` brings
the resolver, `CLAIM device`/`CLAIM mount` bring hardware and data.
A workload that genuinely wants the host (Home Assistant class) either
claims what it touches or is honestly outside cix; the ledger row gets
downgraded accordingly rather than served by a leak.

**The four hard edges, each with a principled channel:**

1. **NSS/identity** (getpwuid under DynamicUser): cix generates a
   minimal synthetic `/etc/passwd`+`/etc/group` for exactly the
   service's identity (and D48d identities) and binds them in — the
   nss_wrapper trick, done by the platform once instead of by every
   pack. Note (systemd.exec read-through): `PrivateUsers=` is
   documented as "particularly useful in conjunction with
   RootDirectory=, as the need to synchronize the user and group
   databases is reduced" to root/nobody/the unit's own user — a userns
   shrinks this edge to a three-line passwd. Mechanism details in §4.2.
2. **DNS + trust roots** (egress services): the *claim* is the
   channel — `CLAIM egress` additionally binds a cix-managed
   resolv.conf (docker's engine-inject move, made declarative). CA
   trust is closure territory (`${pkgs.cacert}`), exactly as FETCH
   already teaches.
3. **Timezone**: `TZ` env; `/etc/localtime` does not exist in the root.
3b. **Logging + notify** (edge found in the systemd.exec read-through):
   the man page warns RootDirectory services cannot reach journald
   "unless the relevant sockets are mounted from the host" — and then
   solves both halves itself: the sd_notify socket is auto-mounted into
   the root when `NotifyAccess=` is set (CIP-79's prober keeps working),
   and `BindLogSockets=` (v257, implied by `MountAPIVFS=yes`) binds the
   journald sockets in. On ≥257 the whole edge dissolves; on older
   systemd it is three explicit `BindReadOnlyPaths=` lines.
4. **Adoption**: phased but without a permanent escape. Phase 1: closed
   root as an *audit gate* — a VM check running every example and
   corpus receipt under the closed root, red = a host dependence found.
   Phase 2 (once the tier is green): closed root becomes the only
   runtime. The phases are eras of the migration, not modes of the
   product.

**The strands intertwine — deliberately.** Closed root only composes
well with software whose dependencies are closure-complete, i.e. with
nix. That couples the isolation thesis to the nix-native thesis:
accepted and embraced (Mathijs), the two reinforce rather than merely
coexist — nix makes closure-truth *available*, closed root makes it
*load-bearing*.

## 4. Open questions

1. **Store posture: closure-only vs whole-store.** Mathijs notes whole
   /nix/store visibility is "of course unproblematic" — true for
   *security* (content-addressed, world-readable by nix's own model),
   but not for *portability*: with the whole store visible, a hardcoded
   out-of-closure path still works-on-my-machine, silently — the exact
   bug class this CIP exists to kill. Closure-only binding is the
   enforcement; its cost is bind-mount count (a big service closure is
   hundreds of paths). Proposal: closure-only, with a measurement in
   phase 1 (unit-file size, mount setup latency vs bwrap's identical
   pattern); fall back to whole-store + a *static* out-of-closure
   reference lint only if the numbers genuinely hurt. Third option from
   the systemd.exec read-through: `RootImage=` — nix *builds* a
   per-item erofs root image of the closure at generation time (one
   loop mount, zero binds), optionally dm-verity-protected via
   `RootHash=`/`RootHashSignature=` — content-addressed,
   integrity-verified root filesystems as store artifacts. Heavier
   machinery, but O(1) in mounts and kernel-enforced integrity;
   evaluate in the same phase-1 measurement.
2. NSS mechanism: generated passwd/group binds (proposed) vs
   `UserDB`/varlink plumbing vs pack-side nss_wrapper convention.
3. Resolver shape under `CLAIM egress`: bind host resolv.conf verbatim,
   or a cix-managed copy (filtered, docker-style)?
4. `--user` degraded mode: closed root there too (user-manager
   RootDirectory works), or is dev-mode exempt (it is already
   "explicitly degraded")? Proposal: same closed root — dev/prod parity
   is worth more than dev convenience here.
5. Ledger consequences: which corpus rows (Home Assistant, Frigate
   pre-CLAIM-device) get honest downgrades in docker.md/corpus.md.

## 5. Decision

Adopted with §3 as proposed and one reversal in §4.1, argued by
Mathijs and conceded: **whole-store ro** is the posture. The
works-on-my-machine claim against it was overstated — nix's reference
scanner makes every store path embedded in the item's bytes (including
manifest argv and env defaults) a closure member *by construction*
("references define dependencies", D32), so the residual escape
surface is only (a) scanner-evading string tricks (pathological; a
static lint candidate) and (b) runtime-supplied store paths from
state/config/env — and for (b) the honest concern is **gc-coherence**
(presence-in-store is gc-revocable; only the rooted closure is not),
which closure-only binding would convert from a rare gc-time surprise
into a deterministic deploy-time failure but not actually fix. That
diagnostic difference does not buy hundreds of bind mounts.
Closure-only binds and the `RootImage=`/dm-verity route are recorded
as available *tightenings* if evidence of class (b) arrives in the
wild — not part of v1.

Other rulings, per the proposals: NSS = generated three-line
passwd/group + `PrivateUsers=` (§3.1); resolver under `CLAIM egress` =
verbatim bind of the host resolv.conf for v1 (works including the
systemd-resolved stub while egress services share the host netns; the
filtered cix-managed copy arrives with the netns era, D49);
`--user` mode gets the same closed root (dev/prod parity); ledger
downgrades (Home Assistant class) land with the implementation.
Phasing as §3.4: audit-gate era first, then the only runtime.
