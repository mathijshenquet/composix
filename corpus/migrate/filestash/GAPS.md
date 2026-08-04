Generated: migrate.md@d582f41 · unknown · 2026-07-31
Status: stale — regenerate with CIP-91

- The page-wide `CPATH`/`LIBRARY_PATH`/`PKG_CONFIG_PATH` preamble predates CIP-88(a): the vendored dev-env snapshot now supplies toolchain search paths from the IMPORTed packages; regeneration verifies the coverage. → case (stale, CIP-88)
- The required coherent static C-library set cannot be selected from the locked nixpkgs packages: `pkgsStatic.giflib` itself fails, but D70 overlays now provide the `.nix` package-customization escape for a regenerated case. → case
- Four helper binaries are linked one by one into `/bin`, the same artifact toolset slop as the larger Wallos pile. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The runtime moved the upstream `/app` tree into `/bin`, `/share`, and `/var/lib/filestash`; mirror the application layout or explain each deliberate split. → prompt
- Go generation is joined to dependency download in a networked `FETCH`, whose generated cache snapshot changed between runs. → evidence
- The Docker receipt followed moving `master` while the Cix side used a recorded revision, so the two probes are not proven against identical source bytes. → evidence
