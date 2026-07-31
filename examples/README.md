# Examples

- `pack/<name>/` holds individual service items. A `Cixfile` is the canonical form.
- `compose/` holds composites that consume pack items through tags.
- `build/` holds build-story projects. `build/ingredient` proves a small EXPECT-pinned HTTPS
  fetch with imported curl and CA roots. `build/projB` proves the Rust FETCH/RUN path;
  `build/projB-chef` proves the same manifest-first dependency workflow. `build/proj1` is
  D57's two-service Rust workspace: one persistent named builder feeds independent API and
  worker artifacts through narrow consumed-path records.

The adoption ladder is: Cixfile for Docker-shaped authoring, `composix.lib.withSpec` for
attaching a manifest to an existing derivation, then plain Nix for the fully native escape
hatch. `pack/redis` intentionally keeps its Cixfile and `default.nix` to show the first two
rungs for the same service. `pack/listenfds` is a second `withSpec` example: its bare v4
manifest demonstrates the same listener contract available in a Cixfile `SERVICE`.

`dstyle/` is a design-era archive.
