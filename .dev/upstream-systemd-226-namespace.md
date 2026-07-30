# Investigation note — UID-map EPERM with DynamicUser + PrivatePIDs + StateDirectory (NOT YET FILED)

Status: do not file from the current evidence; retain as an environment-level investigation note.
Origin: track/composefallback bisection, 2026-07-30 (full evidence trail in
`crates/cix-run/LOG.md`, entries 16:50–16:52 UTC). Composix ships a loud degraded
fallback for this class (D36; `crates/cix-run/src/capabilities.rs`), so filing is
advocacy, not a blocker for us.

## Investigation update — exact failing operation captured

On the stock-systemd-261 NixOS VM, manager debug logging and a PID 1 trace now
identify the exact failing operation. systemd first creates the temporary user
namespace used for the persistent ID-mapped directory. In the PID-namespace child,
it then successfully opens `/proc/2/uid_map` and receives `EPERM` from the write:

```
openat(AT_FDCWD, "/proc/2/uid_map", O_WRONLY|O_NOCTTY|O_CLOEXEC) = 5
write(5, "65534 61222 1\n0 0 1\n", 20) = -1 EPERM (Operation not permitted)
```

`/proc/2` is the `sd-mkuserns` process as seen inside the preceding PID namespace;
the two map entries map overflow UID 65534 to the DynamicUser UID and host root to
namespace root. Thus this is **not** a `setgroups=deny`, GID-map, or
`mount_setattr()` failure: each happens later or on a separate capability probe.
The manager debug journal independently says `Failed to write UID map: Operation
not permitted`, then the generic `Failed to allocate user namespace` wrapper and
`status=226/NAMESPACE`.

## Decisive kernel-axis update — Linux 6.17 also fails in the VM

On 2026-07-30, the NixOS test expression was extended with two otherwise-identical
stock-systemd-261 cells. One pins Linux **6.17.13** through
`boot.kernelPackages`; the other pins the current Linux **6.18.40** package. Both
assert their running kernel and `systemd 261`, then start exactly the minimal
`DynamicUser=yes + PrivatePIDs=yes + StateDirectory=sdbisect` unit.

Both cells fail with exit status 1, `Failed to write UID map: Operation not
permitted`, and `status=226/NAMESPACE`. The test prints the explicit conclusion:

```
kernel axis result: 6.17=uid-map-eperm, 6.18=uid-map-eperm
```

The exact command was:

```sh
nix build .#sdbisect-revert-vm --no-link -L
```

The current primary nixpkgs pin has EOL-disabled its 6.17 alias, so the harness
locks `ef6c19e8baf55f671169995f0fa532511062a99a` (2025-12-19), where
`linuxPackages_6_17.kernel.version` evaluates to 6.17.13. The current NixOS VM
sets its newer boot-module-only options explicitly for that older package; the kernel
and modules themselves come from that pin.

This falsifies the proposed **kernel-only 6.17→6.18 behavior-change** explanation
for the VM reproduction. It does not support a kernel git/lore candidate search or
a kernel-targeted upstream issue. The earlier host success differs from the VM in
more than its kernel: it used the host's real systemd 257 and NixOS/user-space
configuration rather than this current-NixOS QEMU guest; the VM uses QEMU/KVM
virtual hardware and test-runner boot parameters (including
`lsm=landlock,yama,bpf`), its own cgroup/mount topology and dynamic UID allocation,
and newer-NixOS-generated root-unit configuration. The separate v257 VM
compatibility experiment additionally needs current systemd only in the initrd and
compatibility unit placeholders, so it is not a byte-for-byte reconstruction of the
host either. Those differences, or an interaction involving them, remain the live
axis; no one component has been isolated as causal.

## Investigation update — same-host NixOS A/B rejects the candidate

On 2026-07-30, the self-contained two-VM NixOS check was run against systemd 261.
Both VMs use the same minimal root system unit (`Type=oneshot`, `/bin/true`,
`DynamicUser=yes`, `PrivatePIDs=yes`, and `StateDirectory=sdbisect`) and the same
kernel/harness. The stock VM failed with `Failed to allocate user namespace` and
`status=226/NAMESPACE`. The other VM booted the patched systemd store path, where the
`StateDirectory=` ID-mapped-mount caller was changed from
`setgroups_deny=true` to `setgroups_deny=false`, reversing the behavior introduced at
that call site by [`6431c34b8a84`](https://github.com/systemd/systemd/commit/6431c34b8a8487fb50c9cb850bd7d3bf81ad9e2a).
It failed with the same two messages. Thus `6431c34b8a84` is **not confirmed as
causal** by this A/B experiment. The exact command, patched PID 1 store path, and
both VM transcripts are recorded in `.dev/sdbisect.LOG.md`.

The full reverse of 6431c34 does not apply to the Nixpkgs 261 source because it
contains subsequent API/caller changes. The check therefore applies the narrow,
production-relevant functional reversal described above; it leaves subsequent callers
that deliberately choose either setting intact.

## Investigation update — same-harness 257 cell also fails

On 2026-07-30, a third VM was added to the same NixOS test expression. It uses the
same kernel (Linux 6.18.40), VM runner, NixOS configuration, and minimal unit as the
stock 261 cell, but its root-system PID 1 is the systemd 257.6 package from Nixpkgs
revision `0002d4fba62a97fe1260dc41f00deaac9a53f63d` (the test checks both
`systemd 257` and `/proc/1/exe`). Its initrd remains current systemd solely because
current NixOS's initrd module requires unit files introduced after 257; the root
manager and its executable code are 257.6. A small package wrapper supplies inert
unit-file placeholders required by the newer NixOS module, without modifying the
257 manager binary.

The 257 cell **also fails** the exact triple at 226/NAMESPACE. Its debug journal
reaches the same persistent ID-mapped-mount path and reports `Failed to write UID
map: Operation not permitted`. This settles the regression wording: the available
evidence does **not** support a systemd 257→261 regression, or a version-only
systemd cause, on this kernel/harness. The earlier 257 success was on a different
NixOS host and kernel, so the differing environment remains the more likely
explanation. No upstream systemd regression issue should be filed from the current
evidence.

## Investigation update — non-NixOS repro not established

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
command/error trail is in the tracked append-only `.dev/sdbisect.LOG.md`.

Accordingly, the following body must not claim a regression. The non-NixOS repro is
still absent, the same-harness 257 cell fails too, and the NixOS same-host A/B did
not confirm the source candidate. The source analysis below is only a candidate
audit.

## Targeted source audit (v257..v261; not a bisect)

No causal commit is proven. The strongest code-path candidate remains
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
DynamicUser/PrivatePIDs/managed-directory behavior. But the same-harness 257 result
now removes the observed version boundary, so this intent evidence does not support
assigning causality to 6431c34.

## Proposed title (only if a future environment-level upstream issue is warranted)

DynamicUser= + PrivatePIDs= + StateDirectory= fails at 226/NAMESPACE because the
temporary user namespace UID-map write returns EPERM

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

What we have observed:

- Removing **any one** of `DynamicUser=yes`, `PrivatePIDs=yes`, or `StateDirectory=`
  makes the unit start.
- On the same Linux 6.18.40 NixOS VM harness, systemd 257.6 and 261 both fail. The
  257 manager's root PID 1 and version were checked directly. The earlier 257 pass
  was on a different NixOS host/kernel, so this is not evidence of an upstream
  systemd version regression.
- `RuntimeDirectory=` does **not** reproduce (`DynamicUser=yes + PrivatePIDs=yes +
  RuntimeDirectory=` starts fine), further localizing this to **persistent ID-mapped
  managed directories** (`StateDirectory=` is the minimal proven representative; the
  Cache/Logs/Configuration variants share the backing mechanism).
- `user.max_user_namespaces` is not the cause (raising it changes nothing).
- PID 1 strace captures the failure as `write("65534 61222 1\\n0 0 1\\n")` to
  `/proc/2/uid_map` returning `EPERM`, after the open succeeds. This is before the
  `setgroups=deny` or GID-map writes, and not a `mount_setattr()` failure.
- A source audit, not a bisect, identifies 6431c34b8a84 as a historical candidate,
  but a same-host NixOS A/B that disables its `setgroups=deny` behavior at the
  `StateDirectory=` caller still fails. Together with the same-harness 257 failure,
  it is not a supported causal explanation.

The remaining question is environmental: which host/VM configuration or interaction
causes this multi-entry temporary UID-map write to return `EPERM` in the
PID-namespace child? If useful, we can provide the NixOS VM expression and complete
debug/strace trace.

A self-contained NixOS VM test reproducing this is available (we can share the nix
expression; it is derived from `nix/compose-fallback-vm.nix` in our repo).

## Filing notes (ours, not part of the issue)

- Before filing: re-verify against the then-current systemd git main, and search
  issues again — the audit on 2026-07-30 found no existing report.
- Attach or inline a minimal `systemd-run` one-liner repro as well:
  `systemd-run --wait -p DynamicUser=yes -p PrivatePIDs=yes -p StateDirectory=probe /bin/true`
