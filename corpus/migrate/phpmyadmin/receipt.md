# Receipt

Verdict: **check-fail** (Cixfile class: build). The cix probe passed, but the required Docker-mode check did not produce a completed transcript in this round, so this is not claimed as a pass.

Cix transcript:
```
$ ./check.sh cix
BUILDER build step 1 FETCH memo hit df52c5d63888 -> /nix/store/sj2qcy5m4agjwgvg3xi7iydcf9rpzzw2-cix-build-snapshot
BUILDER build step 2 RUN memo hit 679d696a8d00 -> /nix/store/j9mbzi71c3qfvzban74ljsw45w4hy1pw-cix-build-snapshot
status=0
```
Cix store path: `/nix/store/avmjazzz9dal1sd2zwvribiisyp7bj1j-cix-item-phpmyadmin`.

Docker transcript:
```
$ timeout 300 ./check.sh docker
(no completed transcript captured)
```
Docker digest: not produced.
