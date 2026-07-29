# Examples

- `pack/<name>/` holds individual service items. A `Cixfile` is the canonical form.
- `compose/` holds composites that consume pack items through tags.
- `build/` holds build-story projects. `build/projB` proves D39's single Rust FETCH/RUN path;
  `build/projB-chef` proves that copying source after the dependency cook preserves its memo
  entry. `build/proj1` preserves the earlier multi-output build-shape design input.

The adoption ladder is: Cixfile for Docker-shaped authoring, `composix.lib.withSpec` for
attaching a manifest to an existing derivation, then plain Nix for the fully native escape
hatch. `pack/redis` intentionally keeps its Cixfile and `default.nix` to show the first two
rungs for the same service. `pack/listenfds` is a second `withSpec` example: its manifest v3
listener contract has no Cixfile v1 directive.

`dstyle/` is a design-era archive.
