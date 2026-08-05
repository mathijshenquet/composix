Generated: migrate.md@current · gpt-5.6-luna staging, independently rechecked · 2026-08-05
Status: current

- Both twins carry Docker's `STOPSIGNAL SIGQUIT`; nginx writes its pid under declared `RUNDIR /run/nginx`, and the faithful HTTP probe and cold replay pass. → case
- Alpine hooks, fixed uid/gid, nginx.org package selection, stdout symlinks, and exact welcome-page content dissolve into the locked nginx package, systemd identity, and the checked-in HTTP contract. → case
- The copied Alpine hooks' template mutation and cgroup autotuning are declared operator-facing environment inputs but are not exercised by the HTTP probe. → evidence
