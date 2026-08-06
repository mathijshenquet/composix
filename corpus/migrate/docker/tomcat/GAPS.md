Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- The faithful twin uses nixpkgs' Tomcat 10.1.57 tree instead of reproducing the Docker image's separately compiled Tomcat Native library, and it omits the upstream `configtest` assertion that the native library loaded. → case
- `CATALINA_HOME` is `/share/tomcat` rather than upstream `/usr/local/tomcat`; the package tree and Docker cross-image copy are behaviorally close but not path-identical. → case
- Tomcat's `logs`, `temp`, and `work` directories remain under the immutable package assembly even though the Docker writable layer permits runtime writes there. → case
- The bounded HTTP probe accepts the expected empty-server response, proving reachability but no deployed web application behavior. → evidence
- The 2026-08-06 faithful warm/cold commands both exited 0, but verification dirtied the output lock: `sourceHash` changed `4e8b397afdd22a4bc32bf5e1beffd2be13842037a8bbfdbac64df7f809a1ff14` → `a98267fb02f1acf91908f1e3e8f8ae081bae22b9f65e37b7f186dd97a2c5a60a` and `storePath` changed `/nix/store/s58jpph2qgzj18xwwam5is3jkzhqa9mf-cix-item-tomcat` → `/nix/store/5bqhzp9yc7plf621fr33560zs6hdz41v-cix-item-tomcat`. The exact two-line diff was restored byte-for-byte; this is a keying-neutrality exhibit, not regeneration. → language (keying-neutrality wall)
