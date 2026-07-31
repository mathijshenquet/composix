# echo-server migration receipt

Date: 2026-07-30
Layout refreshed: 2026-07-30 (no Cix rerun; the recorded timeout remains live)

Docker image digest (local image ID): `sha256:617137dd0795830b72301249dfbebacb2255fc8614e7eb6952f5ce6c61c53a8d`

Cix item store path: not produced; the Cix build timed out while materializing the npm cache in `FETCH`.

## `./check.sh docker`

```text
docker image sha256:617137dd0795830b72301249dfbebacb2255fc8614e7eb6952f5ce6c61c53a8d
PASS docker
```

Exit status: 0

## `./check.sh cix`

```text
Cloning into '.'...
Note: switching to '2b735482f942cbd889f1d49f3ff892364d0519ac'.

You are in 'detached HEAD' state. You can look around, make experimental
changes and commit them, and you can discard any commits you make in this
state without impacting any branches by switching back to a branch.

If you want to create a new branch to retain commits you create, you may
do so (now or later) by using -c with the switch command. Example:

  git switch -c <new-branch-name>

Or undo this operation with:

  git switch -

Turn off this advice by setting config variable advice.detachedHead to false

HEAD is now at 2b73548 chore(deps): update logstash docker tag to v8.12.0 (#146)
```

Exit status: 124 (`timeout 20` around `cix build .`).

## Corpus fetch verification (2026-07-31)

The raw pinned checkout also contains its Dockerfile. The migration tracks that
Dockerfile separately, so `SOURCE` now excludes it from the fetched build context.
The selected checkout diffed byte-identically with the vendored tree.
