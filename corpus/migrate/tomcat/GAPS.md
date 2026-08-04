Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- Whole packages are copied to ad-hoc `/coreutils`, `/gnused`, `/tomcat`, and `/jre` roots and stitched together with a hand-built `PATH`. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The virtual filesystem diverges from upstream `CATALINA_HOME=/usr/local/tomcat` and its ordinary JRE/package layout; mirror those paths or state why the alternate projection is required. → prompt
- Setup disables Tomcat's private shutdown listener on 8005 to satisfy the declared-port sandbox; retain that deliberate behavioral change as a visible loss. → case
- The bounded probe accepts the expected empty-server 404, proving reachability but no deployed web application behavior. → evidence
- This nixpkgs-only conversion has no Dockerfile-faithful twin. → case: add Dockerfile-faithful twin
