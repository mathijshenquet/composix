Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- A direct source build completed once, producing a static nginx item, but the original `LOGDIR /var/log/nginx` declaration left nginx unable to open its DynamicUser-owned logs. The declaration was removed. → case
- The corrected rebuild's pnpm FETCH emits a huge volatile store/index set and exceeded the independently applied 600-second bound without a final item; its timeout wrapper did not reap the child, so a manual signal ended the supervisor. Runtime after that correction is unproved. → language (lock-scale)
- Docker's Alpine images and fixed user dissolve into nixpkgs nginx and the managed service identity. → case
