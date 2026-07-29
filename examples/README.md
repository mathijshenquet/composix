# Examples

- `pack/<name>/` holds individual service items. A `Cixfile` is the canonical form.
- `compose/` holds composites that consume pack items through tags.
- `build/` holds build-story projects; `build/proj1` preserves the original build-shape example.

The adoption ladder is: Cixfile for Docker-shaped authoring, `composix.lib.withSpec` for
attaching a manifest to an existing derivation, then plain Nix for the fully native escape
hatch. `pack/redis` intentionally keeps its Cixfile and `default.nix` to show the first two
rungs for the same service.

`dstyle/` is a design-era archive.
