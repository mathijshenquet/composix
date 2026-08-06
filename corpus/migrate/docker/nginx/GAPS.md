Generated: migrate.md@current · gpt-5.6-luna staging, independently rechecked · 2026-08-05
Status: current

- Both twins carry Docker's `STOPSIGNAL SIGQUIT`; nginx writes its pid under declared `RUNDIR /run/nginx`, and the faithful HTTP probe and cold replay pass. → case
- The 2026-08-06 faithful warm/cold verification returned the existing item and cold exit 0, but the verification dirtied only the lock's `sourceHash`: `2289625103e7245081b02115293cc8910f4da9520cdb8104152ec153e26dfba0` → `31aa13b1809fbe04ae8957eac7ca84368a76f92cf35dad307e5afb73302fdf93`. The exact diff was restored byte-for-byte; this is a keying-neutrality exhibit, not regeneration. → language (keying-neutrality wall)
- Alpine hooks, fixed uid/gid, nginx.org package selection, stdout symlinks, and exact welcome-page content dissolve into the locked nginx package, systemd identity, and the checked-in HTTP contract. → case
- The copied Alpine hooks' template mutation and cgroup autotuning are declared operator-facing environment inputs but are not exercised by the HTTP probe. → evidence
