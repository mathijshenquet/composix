Generated: migrate.md@d582f41 · unknown · 2026-07-31
Status: stale — regenerate with CIP-82

- Seven sibling `COPY` directives enumerate one deploy output; copy the deploy unit or state exactly what is excluded and why. → prompt
- `LOGDIR /var/log/parse-server` needlessly diverges from upstream `/parse-server/logs`; role-directory paths are free and should mirror the application. → prompt
- Writable `/parse-server/cloud` and `/parse-server/config` were omitted because the old grammar restricted `CONFIGDIR`; CIP-82 removed that restriction, so both upstream paths can now be declared directly. → case
- The runtime moves from Docker's Node 20.19 to nixpkgs Node 22 because the locked universe removed EOL Node 20; retain that explicit compatibility disposition. → case
- The Docker-shaped `ENV NAME=value` spelling is intentionally not grammar today; preserve the canonical spaced form and cite the pending diagnostic disposition. → open question ([`ENV NAME=value`](../../../docs/open-questions.md#proposed-one-line-dispositions-awaiting-mathijs-batch-blessable))
- The item builds, but the Mongo-backed `/parse/health` contract has never passed on the Cix side. → evidence
