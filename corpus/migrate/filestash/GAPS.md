Generated: migrate.md@d582f41 · unknown · 2026-07-31
Status: current

- The page-wide `CPATH`/`LIBRARY_PATH`/`PKG_CONFIG_PATH` preamble is mechanical development-input plumbing that `IMPORT` does not provide. → language ([builder-dev-imports draft](../../../cips/draft/builder-dev-imports.md))
- The required coherent static C-library set cannot be selected from the locked nixpkgs packages: `pkgsStatic.giflib` itself fails and the remaining archives cannot close independently. → language (candidate: customized build-package composition)
- Four helper binaries are linked one by one into `/bin`, the same artifact toolset slop as the larger Wallos pile. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- The runtime moved the upstream `/app` tree into `/bin`, `/share`, and `/var/lib/filestash`; mirror the application layout or explain each deliberate split. → prompt
- Go generation is joined to dependency download in a networked `FETCH`, whose generated cache snapshot changed between runs. → evidence
- The Docker receipt followed moving `master` while the Cix side used a recorded revision, so the two probes are not proven against identical source bytes. → evidence
