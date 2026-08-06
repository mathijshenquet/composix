Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- The faithful 2.0.22 TCP build, MQTT pub/sub roundtrip, and cold replay pass. `WITH_WEBSOCKETS` is disabled because the imported closure did not expose `libwebsockets.h`; WebSockets are not claimed by the TCP probe. → case
- Docker's fixed uid/gid, root-time recursive `chown`, command override, labels, and Alpine entrypoint dissolve into DynamicUser, declared roles, and the checked-in no-auth broker configuration. Operator-supplied configuration remains a compose/materialization concern. → case
- The host drops `PrivatePIDs` while realizing DynamicUser, so Cix reports degraded PID-namespace confinement for the successful MQTT probe. → evidence
- The dissolved twin deliberately follows nixpkgs' Mosquitto version and build flags rather than the faithful 2.0.22 source build. → evidence
- The 2026-08-06 widened-parser sweep could not recreate the local faithful FETCH snapshot because the pinned Ubuntu keyserver response changed: declared `sha256-vSgE3uH3CP8JOW6MW1bZ7CAH6sEglZQHhKQDPqweLNs=`, observed `sha256-t6R8FXIdR0EmuvPy0OTBWvZzkNnmF/y4rNCsyVUr67c=`. → upstream drift wall
