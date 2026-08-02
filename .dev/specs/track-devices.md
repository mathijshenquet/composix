# track/devices — CIP-78: CLAIM gpu / CLAIM device / SHM

Read AGENTS.md first. Authoritative: docs/cips/0078-devices.md §3 + §5
Decision. The CLAIM vocabulary and the `claims` manifest array already
landed (track/claim); this track adds the two device claims and SHM,
their compilation, and the dogfood example. Work in `.worktrees/devices`
on branch `track/devices`. Keep `crates/cix-run/LOG.md` current.

1. **Manifest**: `claims` entries stay bare strings for parameterless
   claims (`"jit"`, `"egress"`, and now `"gpu"`); parametrized claims
   are a one-key object: `{"device": "/dev/ttyUSB0"}`. If the landed
   claims model already has a parametrized precedent, follow that
   instead and record the deviation in the LOG. New top-level def-node
   field `shm: "<size>"` (systemd size syntax, validated at parse).
   Schema strictness as usual: unknown claims/keys rejected with
   spanned errors.
2. **Cixfile**: `CLAIM gpu`, `CLAIM device /dev/<node>` (absolute path
   under /dev required), `SHM <size>` directives; fmt support; parser
   diagnostics in the house style.
3. **Unit compilation** (crates/cix-run/src/unit.rs):
   - Any device claim drops `PrivateDevices=` for that unit and instead
     compiles `DevicePolicy=closed` + `DeviceAllow=` entries — the
     narrow allow-list replaces the blanket private /dev (CIP-78 §3;
     record this mechanical reading in the LOG and docs — the CIP's
     "PrivateDevices otherwise intact" means non-claiming units keep
     today's posture unchanged).
   - `gpu`: `DeviceAllow=/dev/dri rwm` (char class) +
     `SupplementaryGroups=video render`.
   - `device /dev/x`: `DeviceAllow=/dev/x rwm` + the node's owning
     group resolved at generation time (stat the node; fall back to a
     documented warning when the node is absent at generation time —
     generation must not hard-fail on a host without the hardware,
     activation is where absence bites).
   - `shm`: `TemporaryFileSystem=/dev/shm:size=<size>`.
   - `cix run` honors all of this identically (CIP-77).
4. **Compose**: per-service `shm:` override (operator wins, loudly in
   `compose diff`). The `grants:` loosening field is name-reserved by
   CIP-78 but NOT built this track — document the reservation where the
   compose schema is documented. Compose may not silently widen device
   access: no compose-side device grants yet.
5. **Example + scenario**: one new examples/ member (Frigate-shaped:
   `CLAIM device` on a node that exists in the VM, plus `SHM`; gpu claim
   exercised at unit-property level since CI VMs have no GPU). New
   `nix/scenarios/devices.nix` VM check asserting via `systemctl show`:
   `DeviceAllow=`, `DevicePolicy=closed`, `SupplementaryGroups=`,
   absence of `PrivateDevices=`, and from inside the unit: /dev/shm
   mounted at the declared size, the claimed node accessible, an
   unclaimed node NOT accessible. Full Immich/Frigate app dogfood is
   out of scope this track; update docs/corpus.md rows 7 and 17 to
   cite the implemented mechanism honestly (desk grades stay honest —
   do not claim app-level verification).
6. **Docs**: docs/docker.md rows `--device`/`--gpus`/`--shm-size`/
   `--group-add`/tmpfs updated honestly (✅/🔶 with the
   no-privileged story); docs/cixfile.md gains the three directives;
   tour touch only if a shown transcript changes (regen then).
7. **Tests**: parser + fmt round-trip for the new directives; manifest
   schema accept/reject; unit-gen snapshot fixtures (gpu, device, shm,
   combinations, cix run + compose); the VM scenario above.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.
