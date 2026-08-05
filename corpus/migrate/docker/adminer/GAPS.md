Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: stale — regenerate with STOPSIGNAL

This regeneration resolves the earlier version/checksum binder, PHP tuning,
webroot layout, dynamic design/plugin assembly, PHP extension import, artifact
import, and missing-twin findings.

- `ADMINER_DESIGN` and `ADMINER_PLUGINS` use the private `__cix_unset__` sentinel because Cix cannot declare an optional runtime `ENV` with no default. → language (optional ENV declaration)
- Docker requests `STOPSIGNAL SIGINT`; Cix now has the directive, so regenerate this case with that declaration. → stale ([STOPSIGNAL disposition](../../../../cips/dispositions.md#batch-2026-08-04-blessed-by-mathijs-dockermd-application-queued))
- The worker's warm build passed, but the independent fresh fetch is EXPECT-hostile: its published file checksum passes while Cix reports declared `sha256-bJPK…` versus fetched `sha256-XJVI…`, leaving no snapshot for cold replay; keep the pin unchanged and normalize the fetch in the volatile-fetch fix round. → case (cold stability)
