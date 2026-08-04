Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: stale — regenerate with CONFIGDIR path freedom

- The locked universe's supported Node 22 replaces Docker's exact Node 20.19/Alpine identity. → case
- The service listens on 18091 for the repository probe rather than Docker's default 1337, and its config role moves from `/parse-server/config` to `/etc/parse-server` because the current runner rejects CONFIGDIR outside `/etc`. → language ([recorded CONFIGDIR restriction](../../../../cips/open-questions.md#open-for-agents))
- The cross-builder `COPY ${deps}/node_modules` is cold-divergent even though the warm build and Mongo-backed probe pass; this should be diagnosed and normalized by [CIP-87's cold-divergence machinery](../../../cips/accepted/0087-read-set-keying.md). → language (cold divergence audit)
- Independent ordinary dependency fetches alternated between the staged `prod_node_modules` hash and a rejected different hash before the successful probe, so the fetched output is unstable even before the explicit cold comparison; do not repin this variance away. → case (cold stability)
