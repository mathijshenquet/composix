# Devices, GPU, and shared memory

Status: draft, amended 2026-08-01 after Mathijs's review — narrowed to
`GRANT gpu` + `SHM`, ready to adopt. Deliberately dogfood-gated (the
manifest v2 deferral said "needs a dogfood case" — this doc picks one).

## 1. The problem

Hardware access has no cix spelling: Immich wants a GPU for ML, Frigate
wants five device passthroughs + sized shm, pytorch builds want CUDA,
Home Assistant wants USB. Docker spells these `--device`, `--gpus`,
`--shm-size`, `--group-add`; our ledger defers all four. The capability
thesis (D20a) says grants must be semantic and narrow — `--privileged`
stays refused — so the question is which narrow grants to mint and on
which side (manifest vs compose) they live.

## 2. Prior work

**Docker** `--device` allowlists a node in the device cgroup and mknods
it in; `--gpus` delegates to the nvidia-container-toolkit, whose real
job is mounting *matching userspace driver libraries* into the
container — the container/host driver-version split is the entire
hardness of GPU-in-docker. That pain birthed **CDI** (Container Device
Interface): a vendor-neutral JSON description of "device = nodes +
mounts + env + hooks", now the standard shape in the OCI world.
**Kubernetes** wraps the same via device plugins / DRA: the pod
*requests* a named resource (`nvidia.com/gpu: 1`), the node supplies it
— need/supply split again.

**systemd**: `DeviceAllow=` (cgroup device filtering) plus
`PrivateDevices=` (our default posture) plus `SupplementaryGroups=`
(render/video/dialout — how bare-metal Linux has always granted device
access). **NixOS dissolves the GPU library problem**: host kernel driver
and userspace CUDA/VA-API stacks come from one config; a cix item's
closure can depend on the exact userspace matching the host — there is
no image/host split to bridge, which is most of what
nvidia-container-toolkit exists to do.

**shm**: docker `--shm-size` resizes `/dev/shm` (postgres, chromium; 5
of 18 corpus files set it). systemd has no direct knob, but
`TemporaryFileSystem=/dev/shm:size=1g` mounts a sized private tmpfs at
exactly that path — native mechanism, arguably better (private by
construction).

## 3. Recommendation

Mint **one semantic grant** now, in the existing grant vocabulary
(`GRANT egress`, `GRANT jit` — review aligned the spelling), dogfood-gated
on the Immich-shaped example (corpus §5 — GPU + shm in one package):

- `GRANT gpu` → the render case: `DeviceAllow=/dev/dri rwm` (char
  class), `SupplementaryGroups=video render`, `PrivateDevices` stays on
  otherwise. Vendor userspace is the item's problem via nixpkgs (the
  NixOS dissolution above); no toolkit, no CDI machinery until a
  non-NixOS-host case forces it. Pareto per review: multi-GPU node
  addressing (the 2×H100+2×A100 class) is real but breaks the
  pack/compose boundary — deliberately out until the single-grant form
  is bitten; expect node selection to arrive as a compose-side override
  when it does.

The explicit-node case (`GRANT device /dev/ttyUSB0` — Frigate's coral,
HA's zigbee stick) is sketched but **YAGNI'd until we run into it**
(review): same mechanics, minted on first real need.

Polarity per D49(a): the manifest declares the need (app knowledge);
compose may tighten silently or loosen loudly. `--group-add` dissolves
into grants (groups are an implementation detail of device access, not
a user-facing knob). `--privileged` stays ❌ even as a compose override —
the diagnostic escape hatch is running the thing outside cix, honestly.

**`SHM <size>`** becomes a manifest field (app knowledge — postgres
documents its needs) with compose override, compiled to
`TemporaryFileSystem=/dev/shm:size=`. This also answers the ledger's
tmpfs row: arbitrary tmpfs destinations keep waiting; `/dev/shm` is the
demanded 90%.

## 4. Open questions — resolved in review

1. One grant, no node addressing (pareto; multi-GPU waits for the bite).
2. Frigate-class hardware YAGNI'd until we run into it.
3. `cix run` honors `GRANT gpu` directly — manifest-side, and cix run is
   degenerate unary compose anyway.

## Changelog

- 2026-08-01: drafted; amended after review — spelling aligned to
  `GRANT gpu`, explicit-node grant and Frigate round YAGNI'd, run
  behavior settled.
