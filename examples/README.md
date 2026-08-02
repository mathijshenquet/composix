# Examples

- `pack/<name>/` holds individual service items. A `Cixfile` is the canonical form.
- `pack/devices/` is the Frigate-shaped device/SHM example; its `/dev/cix-device`
  claim is exercised by the NixOS device scenario rather than requiring hardware on the host.
- `compose/` holds composites that consume pack items through tags.
- `build/` holds build-story projects. `build/ingredient` proves a small EXPECT-pinned HTTPS
  fetch with imported curl and CA roots. `build/projB` proves the Rust FETCH/RUN path;
  `build/projB-chef` proves the same manifest-first dependency workflow. `build/proj1` is
  D57's two-service Rust workspace: one persistent named builder feeds independent API and
  worker artifacts through narrow consumed-path records. `build/from-item` is the small D65
  cross-item COPY demonstration: tag the nginx example, then consume its configuration through
  a lock-pinned FROM binder. `build/item` is D68's pure asset-tree example: it emits no manifest
  and is meant to be tagged or consumed through FROM rather than run.

The adoption ladder is: Cixfile for Docker-shaped authoring, `composix.lib.withSpec` for
attaching a manifest to an existing derivation, then plain Nix for the fully native escape
hatch. `pack/redis` intentionally keeps its Cixfile and `default.nix` to show the first two
rungs for the same service. `pack/listenfds` is a second `withSpec` example: its bare v4
manifest demonstrates the same listener contract available in a Cixfile `SERVICE`.

`dstyle/` is a design-era archive.
