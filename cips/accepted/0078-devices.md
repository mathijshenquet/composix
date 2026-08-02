# Devices, GPU, and shared memory

Status: **CIP-78, adopted 2026-08-01** (Mathijs: "CLAIM is akkoord").
Decision in §5. Deliberately dogfood-gated (the manifest v2 deferral
said "needs a dogfood case" — this CIP picks Immich + Frigate).

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

Mint **two claims** (keyword pending §4.1; CLAIM used here), dogfooded
on the Immich-shaped example (corpus §5) with Frigate (corpus row 17) as
the second consumer:

- `CLAIM gpu` — the semantic form. Mechanically today it IS sugar for
  the render stack: `DeviceAllow=/dev/dri rwm` (char class) +
  `SupplementaryGroups=video render`, `PrivateDevices` otherwise intact.
  It stays a named *need* rather than a node spelling because (a) the
  compiler owns the class→groups mapping, (b) it leaves room for node
  addressing later without new vocabulary (multi-GPU — the
  2×H100+2×A100 class — arrives as a compose-side override on the same
  claim when it bites; pareto per review), and (c) it names what
  migration tables can target.
- `CLAIM device /dev/ttyUSB0` — the literal form (Frigate's coral, HA's
  zigbee stick, V4L2 cameras): `DeviceAllow=` for the node + its owning
  group (`dialout`, `video`, …) resolved at generation time.

Polarity per D49(a): the manifest *claims* the need (app knowledge);
compose may tighten silently or loosen loudly. `--group-add` dissolves
into claims (groups are an implementation detail of device access, not
a user-facing knob). `--privileged` stays ❌ even as a compose override —
the diagnostic escape hatch is running the thing outside cix, honestly.

**`SHM <size>`** becomes a manifest field (app knowledge — postgres
documents its needs) with compose override, compiled to
`TemporaryFileSystem=/dev/shm:size=`. This also answers the ledger's
tmpfs row: arbitrary tmpfs destinations keep waiting; `/dev/shm` is the
demanded 90%.

## 4. Open questions

1. **CLAIM vs GRANT** (Mathijs's question; analysis): manifest-side the
   pack *requests* — it cannot grant itself anything, so `GRANT` in a
   Cixfile has the polarity backwards (SQL `GRANT` is the DBA speaking;
   k8s spells the workload side `resources.requests`, and for devices
   literally **ResourceClaim** in DRA). Recommendation: **CLAIM**, and
   rename the existing `GRANT egress`/`GRANT jit` in the same sweep
   (alpha, D72, cheap famtags-style rename) so there is one vocabulary.
   Bonus: the compose-side loosening field can then honestly be called
   `grants:` — the manifest CLAIMs, the operator GRANTs; the vocabulary
   teaches the D49(a) polarity by itself.
2. ~~Frigate~~ — back in scope (it was already corpus row 17); the
   literal `CLAIM device` form ships in the same round.
3. ~~cix run~~ — honors claims directly (CIP-77).
4. `SHM` confirmed in ("ook niet verkeerd om erin te hebben").

## 5. Decision

**`CLAIM`** is the manifest keyword: `CLAIM gpu`, `CLAIM device
/dev/<node>`, and the existing `GRANT egress`/`GRANT jit` rename to
`CLAIM egress`/`CLAIM jit` in one sweep (alpha, D72). The compose-side
loosening field is reserved the name `grants:` — the manifest CLAIMs,
the operator GRANTs; the vocabulary teaches the D49(a) polarity. `SHM
<size>` manifest field with compose override, compiled to
`TemporaryFileSystem=/dev/shm:size=`. `--privileged` stays refused in
every layer; `--group-add` dissolves into claims. `cix run` honors
claims directly (CIP-77).

## Changelog

- 2026-08-01: drafted; r1 after review — spelling to grant vocabulary,
  Frigate YAGNI'd. r2 same day — YAGNI reversed (Frigate is corpus row
  17), `CLAIM` analysis added with rename-sweep recommendation,
  `CLAIM gpu`-as-sugar clarified (mechanically /dev/dri+groups today,
  semantically a named need). Adopted same day as CIP-78.
