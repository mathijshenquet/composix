# Apache HTTPD migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh httpd`
restored `docker-library/httpd` at
`9b63b37c9be3b42d16b924ebb40af3cc40793119`, context `2.4/alpine`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0. It built the
faithful 2.4.68 source translation, started it through the system manager, and
value-checked an HTTP response containing `It works!`.

`target/debug/cix build --cold corpus/migrate/docker/httpd#httpd` exited 1
after the source build because the warm trace recorded
`output/usr/local/apache2/conf/extra/sedAsylf1` as `Some(Absent)` and the cold
trace recorded `None`. The result is recorded as a CIP-87 read-set defect; it
does not invalidate the successful behavior probe.

The dissolved nixpkgs twin built warm and cold with exit 0, producing
`/nix/store/ggpg5klb6p12d0xhhk6yx9ws6klj46b0-cix-item-httpd`.

## 2026-08-06 self-write-trace regeneration

`./check.sh cix` exited 0, starting the faithful service and value-checking an
HTTP response containing `It works!`.

After clearing the prior memo and workspace state while retaining the pinned
FETCH inputs, this synchronous warm command exited 0:

```text
env CIX_STATE_DIR=$PWD/.dev/scratch/httpd-regen/state CIX_BUILD_WORKSPACE_DIR=$PWD/.dev/scratch/httpd-regen/workspaces target/debug/cix build corpus/migrate/docker/httpd#httpd
```

It produced `/nix/store/3zgq560rmcq6hs9i4p1z2hq5s8dznr23-cix-item-httpd`.
The paired synchronous cold command also exited 0 and produced the same item:

```text
env CIX_STATE_DIR=$PWD/.dev/scratch/httpd-regen/state CIX_BUILD_WORKSPACE_DIR=$PWD/.dev/scratch/httpd-regen/workspaces target/debug/cix build --cold corpus/migrate/docker/httpd#httpd
```

The lock changed from 124,383 lines to 38,562 lines (delta `-85,821`), with
final SHA-256
`67f26d3a2e165a94e5a9264e04d84c03a3ea1e7d86b065380517a8b1dbd4a1fd`.
The lock's BUILD read set no longer contains the generated object paths that
previously varied between warm and cold.
