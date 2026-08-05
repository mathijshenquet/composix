Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: stale — regenerate with CIP-96

This regeneration resolves the earlier version/checksum binder, PHP tuning,
webroot layout, dynamic design/plugin assembly, PHP extension import, artifact
import, and missing-twin findings.

- Docker requests `STOPSIGNAL SIGINT`; Cix has no service stop-signal directive, so systemd's normal termination semantics remain authoritative. → language ([recorded stop-signal gap](../../../../cips/open-questions.md#proposed-one-line-dispositions-awaiting-mathijs-batch-blessable))
- The worker's warm build passed, but the independent fresh fetch is EXPECT-hostile: its published file checksum passes while Cix reports declared `sha256-bJPK…` versus fetched `sha256-XJVI…`, leaving no snapshot for cold replay; keep the pin unchanged and normalize the fetch in the volatile-fetch fix round. → case (cold stability)
