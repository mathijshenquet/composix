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
