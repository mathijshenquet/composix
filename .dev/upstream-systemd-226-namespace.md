# Upstream issue draft — systemd 261 regression (NOT YET FILED)

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

## Proposed title

systemd 261 regression: DynamicUser= + PrivatePIDs= + StateDirectory= fails at
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

Observations:

- Removing **any one** of `DynamicUser=yes`, `PrivatePIDs=yes`, or `StateDirectory=`
  makes the unit start.
- The equivalent workload worked on systemd 257.
- `RuntimeDirectory=` does **not** reproduce (`DynamicUser=yes + PrivatePIDs=yes +
  RuntimeDirectory=` starts fine), further localizing this to **persistent ID-mapped
  managed directories** (`StateDirectory=` is the minimal proven representative; the
  Cache/Logs/Configuration variants share the backing mechanism).
- `user.max_user_namespaces` is not the cause (raising it changes nothing); both v257
  and v261 create a temporary user namespace while applying the ID-mapped
  managed-directory mount, so the causal commit was not pinpointed by our audit of
  258–261 changes.

Question: is this an unintended interaction between PID-namespace setup (`sd-pidns`)
and the user namespace used for ID-mapped managed directories?

A self-contained NixOS VM test reproducing this is available (we can share the nix
expression; it is derived from `nix/compose-fallback-vm.nix` in our repo).

## Filing notes (ours, not part of the issue)

- Before filing: re-verify against the then-current systemd git main, and search
  issues again — the audit on 2026-07-30 found no existing report.
- Attach or inline a minimal `systemd-run` one-liner repro as well:
  `systemd-run --wait -p DynamicUser=yes -p PrivatePIDs=yes -p StateDirectory=probe /bin/true`
