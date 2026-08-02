# Mastodon-shaped stack migration receipt

Source revision: `ad245adf510cf56953ebb8a7dfc5db16c0f58403`
(Mastodon main on 2026-08-02).

This conversion uses the pre-agreed faithfully-shaped fallback. PostgreSQL and
Redis are the real nixpkgs services. `web` and `sidekiq` are small
Puma/worker-shaped Python processes, while `streaming` uses real nginx to retain
the long-running HTTP and health behavior without compiling Mastodon's Rails,
Node, and asset trees. The sixth `cleanup` member represents a native scheduled
maintenance command; the upstream compose file itself has five active services.

The executable check is intentionally stronger than a schema-only migration. It
builds and tags every member, activates the complete compose tree, and proves:

- PostgreSQL and Redis become ready before the four structural consumers use
  their declared Unix-socket edges.
- `cix up` remains blocked through the web member's deliberate three-second HTTP
  readiness delay. Every long-running member has a declared readiness probe;
  web and sidekiq also exercise native-notify liveness and its restart policy.
- PostgreSQL initializes from `db-password`, and each database consumer reads
  the same file from `$CREDENTIALS_DIRECTORY`; no secret value enters a Cixfile,
  compose file, store path, or journal record.
- web and sidekiq independently write the same compose-owned `public-system`
  shared-rw surface.
- the native five-second persistent timer fires the maintenance app.
- `cix logs corpus-mastodon/web` contains the web marker and excludes the
  sidekiq marker.

## Honest losses

Mastodon's `internal_network: { internal: true }` segmentation is not expressed:
this track is fenced from the concurrent networking implementation and therefore
uses released host networking. Unix edges remove ambient IP access for the
PostgreSQL and Redis data paths, but they do not pretend to enforce D26/D27
network segmentation. The worker and HTTP stubs verify composition behavior,
not Mastodon application correctness, asset compilation, federation, or upgrade
semantics. `CLAIM egress` appears only on web and sidekiq, the two shapes that
initiate federation and remote-media work; database, Redis, streaming, and local
cleanup do not receive it.

The first live round also found a systemd-257 compatibility boundary in the
cix-owned HTTP/TCP liveness adapter: its `ExecStartPost` parent exits zero after
forking the resident pinger, but the pinger is not retained on this host and the
healthy service later hits `WatchdogSec`. The accepted health VM passes this
mechanism on newer systemd. This track does not change product code; it records
the finding and uses CIP-79's equally native application-notify path for the two
stub processes while keeping cix-owned HTTP/TCP readiness adapters on all five
long-running members.

## Synchronous receipt

`./check.sh cix` exited 0 on 2026-08-02. Its final items were:

```text
postgres  /nix/store/sgi81gdjb7y91plmq60bg76k5z32crqn-cix-item-postgres
redis     /nix/store/94pyzpkfryrjbxm92mrp8qa2h3dbhdsa-cix-item-redis
web       /nix/store/afc7xdw294x7vhk6q6rripry1xiys3vk-cix-item-web
sidekiq   /nix/store/p0aafmd6j6mvrx409avj5xpahrnykw6m-cix-item-sidekiq
streaming /nix/store/lpwp6923950n3rkv32flld7wdmqlnqw0-cix-item-streaming
cleanup   /nix/store/wf46a8q03rcjm72nqbzy9n3xpl7f6hh7-cix-item-cleanup
```

The compose check reported six services and two Unix edges. During activation,
the web readiness adapter logged three refused connections at one-second
intervals while the application deliberately delayed; `cix up` returned only
after `mastodon web ready`. The shared host surface then held the exact web and
sidekiq backend receipts, the credential marker named
`CREDENTIALS_DIRECTORY`, and the timer journal contained three cleanup firings.
The web-only `cix logs` query contained no sidekiq marker. Both native-notify
services remained active after a seven-second wait, exceeding their six-second
watchdog window. The check printed
`PASS cix: shared-rw, readiness, credential, timer, and member logs` and its
EXIT cleanup completed with status 0, including `cix down --purge --yes`.

The final rerun used `devenv shell -- ./corpus/migrate/mastodon/check.sh cix`
and exited 0 synchronously. It refreshes the `pkgs` lock before every member
build so a locally cached item cannot hide a garbage-collected external runtime
path. A second refresh of all six `Cixfile.lock` files produced no byte changes.

## Closed-root receipt

`devenv shell -- nix build
.#checks.x86_64-linux.scenario-closedroot-audit -L` exited 0 synchronously on
2026-08-02. The exhaustive audit classifies Mastodon as audited and reproduces
the complete six-member compose rather than reducing it to unary items. It
verifies sealed roots for every member, with no ambient `/bin/sh`; PostgreSQL's
`initdb` shell dependency is the sole exception and is an explicit read-only
item mount. Redis declares the C locale instead of inheriting host locale data.

The VM also proves the exact `LoadCredential=` unit contract and behavioral
credential use, PostgreSQL and Redis Unix-edge ordering, both HTTP endpoints,
the shared-rw markers from web and sidekiq, watchdog survival, repeated cleanup
timer execution, member-scoped logs, and removal of shared state and all six
closed-root directories after `down --purge`.
