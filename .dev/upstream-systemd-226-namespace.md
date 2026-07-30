# Upstream issue draft — possible systemd 261 regression (NOT YET FILED)

Status: draft, awaiting Mathijs's go to file at https://github.com/systemd/systemd/issues.
Origin: track/composefallback bisection, 2026-07-30 (full evidence trail in
`crates/cix-run/LOG.md`, entries 16:50–16:52 UTC). Composix ships a loud degraded
fallback for this class (D36; `crates/cix-run/src/capabilities.rs`), so filing is
advocacy, not a blocker for us.

## Investigation update — non-NixOS repro not yet established

The requested upstream same-harness A/B could **not** be completed on this host, so
the v257-pass/v261-fail result remains cross-harness and NixOS-only. This is not
evidence that the issue is NixOS-specific (or that it is not); no non-NixOS guest
ever booted.

The exact attempt used a clean upstream systemd clone at v261 in
`/home/mathijs/tmp/systemd-sdbisect`, current upstream mkosi in
`/home/mathijs/tmp/mkosi-sdbisect`, and mkosi's Fedora/rawhide main image:

```sh
sudo -n /nix/var/nix/profiles/default/bin/nix shell \
  nixpkgs#mkosi nixpkgs#qemu nixpkgs#dnf5 nixpkgs#rpm nixpkgs#createrepo_c -c \
  sh -c 'PATH=/home/mathijs/tmp/sdbisect-bin:$PATH; exec \
  /home/mathijs/tmp/mkosi-sdbisect/bin/mkosi -f --tools-tree= \
  --repository-key-check=no --distribution fedora --release rawhide build'
```

The NixOS host denies mkosi's unprivileged namespace setup, so the build required
root. The packaged mkosi lacks the Git metadata needed by this systemd revision's
`MinimumVersion=commit:…` check; current upstream mkosi was used instead. The Fedora
bootstrap also required nixpkgs `dnf5`, `rpm`, and `createrepo_c` (with a local
`dnf`→`dnf5` wrapper). systemd v261's RPM compilation completed, but final initrd
assembly failed before boot: its `systemd` and `systemd-udev` `%sysusers` scriptlets
reported `usermod: cannot lock /etc/passwd; try again later`, and mkosi's `dnf5
--installroot=/buildroot … install … systemd … udev` exited 1. The full append-only
command/error trail is in `.dev/sdbisect.LOG.md` (intentionally untracked).

Accordingly, the following body must not claim a regression until the same-harness
A/B has been reproduced. The source analysis below is only a candidate audit.

## Targeted source audit (v257..v261; not a bisect)

No causal commit is proven: the required behavior test never ran, so `git bisect`
would only bisect host/mkosi failures. The strongest code-path candidate is
[`6431c34b8a84`](https://github.com/systemd/systemd/commit/6431c34b8a8487fb50c9cb850bd7d3bf81ad9e2a),
`namespace-util: make "setgroups" users property writable via userns_acquire()`.
It is between v257 and v261. Before it, `userns_acquire()` created the temporary
user namespace for an ID-mapped mount, wrote its UID map and GID map, and returned
its namespace FD. The commit inserts a `/proc/<child>/setgroups = deny` write between
those operations and changes the persistent managed-directory call site in
`src/core/namespace.c` to request it:

```c
userns_fd = userns_acquire(uid_map, gid_map, /* setgroups_deny= */ true);
```

That is the exact helper whose returned error is wrapped as `Failed to allocate user
namespace` while applying the ID-mapped `StateDirectory=` mount. It also fits the
observed boundary: persistent DynamicUser directories use this path, while the
RuntimeDirectory-only case does not. The commit message says it enables the operation
for unprivileged namespaces and that enabling it for all existing users, including
ID-mapped mount users, “doesn't hurt”. It supplies no test for the DynamicUser +
PrivatePIDs + persistent-directory triple.

The PID namespace audit found the large
[`8234cd9989d`](https://github.com/systemd/systemd/commit/8234cd9989d3834bf5c06e2b597ec097b985e1e8)
`DelegateNamespaces=` refactor and its
[`38748596f0`](https://github.com/systemd/systemd/commit/38748596f0783f2b773bd95d4af4d83f5b5ff872)
user-manager follow-up. They deliberately split namespace setup before and after a
unit user namespace. For the observed root system unit with default
`DelegateNamespaces=`, however, the PID namespace remains non-delegated and is still
set up before the mount namespace, as in v257. The smaller
[`698ac172aa`](https://github.com/systemd/systemd/commit/698ac172aadd15afced079bb9553e1ea24e63d06)
only detaches/reparents the PID-namespace child. These remain contextual candidates,
not a causal finding.

Neither the v258–v261 NEWS entries nor 6431c34's message announce an incompatible
DynamicUser/PrivatePIDs/managed-directory behavior. If the same-harness result is
eventually confirmed, the available intent evidence points to a bug in (or an
uncovered interaction introduced by) 6431c34 rather than an intended behavior change.
For now, that assessment is conditional.

## Proposed title

Possible systemd regression: DynamicUser= + PrivatePIDs= + StateDirectory= fails at
226/NAMESPACE ("Failed to allocate user namespace")

## Proposed body

On a NixOS test VM with systemd 261 and Linux 6.18.40, a root **system** service
containing only:

```ini
[Service]
Type=oneshot
ExecStart=/bin/true
DynamicUser=yes
PrivatePIDs=yes
StateDirectory=probe
```

fails before exec with:

```
Failed to allocate user namespace: Operation not permitted
...: /var/lib/private/probe
status=226/NAMESPACE
```

What we have observed (not yet same-harness A/B evidence):

- Removing **any one** of `DynamicUser=yes`, `PrivatePIDs=yes`, or `StateDirectory=`
  makes the unit start.
- The equivalent workload worked on systemd 257 on a NixOS host, whereas the failure
  was observed in a NixOS test VM. We have not yet reproduced either result in the
  same non-NixOS harness, so please do not read this as a proven upstream regression.
- `RuntimeDirectory=` does **not** reproduce (`DynamicUser=yes + PrivatePIDs=yes +
  RuntimeDirectory=` starts fine), further localizing this to **persistent ID-mapped
  managed directories** (`StateDirectory=` is the minimal proven representative; the
  Cache/Logs/Configuration variants share the backing mechanism).
- `user.max_user_namespaces` is not the cause (raising it changes nothing).
- A source audit, not a bisect, identifies 6431c34b8a84 as the strongest candidate:
  it adds a `setgroups=deny` write to the temporary user namespace used by the
  ID-mapped managed-directory mount. We have not yet captured which low-level operation
  returns `EPERM` in the failing VM.

Could this be an unintended interaction between the `setgroups=deny` step in the
temporary ID-mapped-mount user namespace and PID-namespace/proc setup? If useful, we
can provide the NixOS VM expression and a debug/strace trace from a rerun.

A self-contained NixOS VM test reproducing this is available (we can share the nix
expression; it is derived from `nix/compose-fallback-vm.nix` in our repo).

## Filing notes (ours, not part of the issue)

- Before filing: re-verify against the then-current systemd git main, and search
  issues again — the audit on 2026-07-30 found no existing report.
- Attach or inline a minimal `systemd-run` one-liner repro as well:
  `systemd-run --wait -p DynamicUser=yes -p PrivatePIDs=yes -p StateDirectory=probe /bin/true`
