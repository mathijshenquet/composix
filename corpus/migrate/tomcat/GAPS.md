Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- The faithful twin uses nixpkgs' Tomcat 10.1.57 tree instead of reproducing the Docker image's separately compiled Tomcat Native library, and it omits the upstream `configtest` assertion that the native library loaded. → case
- `CATALINA_HOME` is `/share/tomcat` rather than upstream `/usr/local/tomcat`; the package tree and Docker cross-image copy are behaviorally close but not path-identical. → case
- Tomcat's `logs`, `temp`, and `work` directories remain under the immutable package assembly even though the Docker writable layer permits runtime writes there. → case
- The bounded HTTP probe accepts the expected empty-server response, proving reachability but no deployed web application behavior. → evidence
