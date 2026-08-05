# HAProxy migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh haproxy`
restored `docker-library/haproxy` at
`9dfb33f798390d8d9b64041834acd72f1591de48`, context `3.2/alpine`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0 after faithfully
compiling HAProxy 3.2.22; `haproxy -v` contained the value `HAProxy version
3.2.22`. `target/debug/cix build --cold
corpus/migrate/docker/haproxy#haproxy` also exited 0 and produced
`/nix/store/h0n4r9i2c13sciwmv7w26qfzbzgbfj96-cix-item-haproxy`.

Formatting is separately and deliberately red: `cix fmt --check` reports the
required indentation. A temporary copy first built successfully unformatted;
after `cix fmt`, its cold build exited 1 because the same FETCH had a new
identity and no snapshot at
`3378f6418827b7c769e19aefc1f52f90dce578bebce743ab98056cd4c5e2336d`.
The formatter is therefore not applied to this locked Cixfile; the exact
reproduction is promoted in `cips/draft/fmt-key-neutrality.md`.

The dissolved nixpkgs twin built warm and cold with exit 0, producing
`/nix/store/43s99f6rhw30h8k3kkwqvd8ikm9679qp-cix-item-haproxy`.
