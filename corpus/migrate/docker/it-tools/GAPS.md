Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: stale — regenerate with CIP-99 workspace-root aggregation

- A direct source build completed once, producing a static nginx item, but the original `LOGDIR /var/log/nginx` declaration left nginx unable to open its DynamicUser-owned logs. The declaration was removed. → case
- The corrected rebuild's pnpm FETCH emits a huge volatile store/index set and exceeded the independently applied 600-second bound without a final item; its timeout wrapper did not reap the child, so a manual signal ended the supervisor. Runtime after that correction is unproved. → language (lock-scale)
- The staged lock retains descendants below a fully observed FETCH workspace root because the original CIP-99 aggregation helper did not treat `.` as a root. That criterion is fixed, but this lock may be regenerated only after a completed corrected build; the later pnpm trace is partial/volatile and must remain per-path. → language (lock-scale)
- Docker's Alpine images and fixed user dissolve into nixpkgs nginx and the managed service identity. → case
