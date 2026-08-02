# track/closedroot — CIP-84 phase 1: closed root as audit gate

Read AGENTS.md first (synchronous receipts; herdr worktree). 
Authoritative: cips/accepted/0084-closed-root.md §3 (the four hard
edges) + §5 Decision (whole-store ro; PrivateUsers three-line passwd;
verbatim resolv.conf bind under CLAIM egress; --user parity; phasing).
Work in `/home/mathijs/worktrees/composix/track-closedroot` (herdr
worktree) on branch `track/closedroot`. Keep `crates/cix-run/LOG.md`
current. Nothing else is in flight — you own unit generation.

1. **Closed-root compilation** (phase 1: behind `--closed-root` on
   `cix run`/compose generation, default OFF this track — phase 2
   flips it):
   - `RootDirectory=` on an empty per-unit root + `MountAPIVFS=yes`;
   - `/nix/store` bind, read-only, WHOLE (§5; no closure-only binds);
   - the item's D22 projections at their declared absolute paths;
   - role dirs (CIP-82 machinery) + claims-derived extras (shared
     surfaces, devices, host/shared materializations) — inside the
     closed root, composing with the existing mirror/bind machinery;
   - `/usr/bin/env` symlink resolving INTO the closure (dangling
     unless an env provider is in it — D58-analogue diagnostic);
     **no /bin/sh ever** (teaching error text per CIP-80: the shell
     is a named dependency);
   - NSS: generated three-line `/etc/passwd`+`/etc/group` for exactly
     the unit's identity (+ D48d static identities), bound in;
     `PrivateUsers=` per the §3.1/systemd.exec note;
   - `CLAIM egress` → verbatim bind of host `/etc/resolv.conf` (the
     v1 ruling); CA trust stays closure territory — document;
   - timezone: `TZ` env only, no `/etc/localtime`;
   - notify/journald: `NotifyAccess`-triggered auto-mount covers the
     prober; on systemd ≥257 rely on `MountAPIVFS`-implied
     `BindLogSockets=`, else three explicit `BindReadOnlyPaths=`
     lines (version-guard with a comment naming the man-page edge);
   - `--user` mode: same closed root (dev/prod parity, §5), degrading
     through the existing capability-probe machinery where the user
     manager cannot (D13 pattern, devfix precedent).
2. **The audit gate** (the actual phase-1 deliverable): a new
   `nix/scenarios/closedroot-audit.nix` VM check that runs EVERY
   examples/ pack member and every corpus/migrate case with a cix
   artifact under `--closed-root`, asserting start + the case's own
   health/probe where one exists. Red = a found host dependence: fix
   it in the pack (claim what it touches) where legitimate, or record
   the honest ledger downgrade (Home Assistant class) in
   docs/corpus.md/docs/docker.md per the ledger convention. Every
   downgrade decision goes in the LOG with its reasoning.
3. **Docs**: docs/cixfile.md closed-root section (the sealed-box
   thesis, the four edges, the no-/bin/sh rule); docs/docker.md rows
   this makes honest (ProtectSystem leak row → closed); ledger
   downgrades from item 2.
4. **Tests**: unit-gen snapshot fixtures (closed root × claims ×
   user/system × egress/devices/dirs combinations); NSS passwd/group
   content tests; the audit VM scenario; existing scenarios must stay
   green with the flag OFF (default unchanged this track).

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. All receipts synchronous.
Commit on this branch when green.
