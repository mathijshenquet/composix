Generated: CIP-102 volatile-fetch sweep + CIP-96 ENV grammar + STOPSIGNAL · 2026-08-05
Status: stale — regenerate with STOPSIGNAL

This regeneration resolves the earlier version/checksum binder, PHP tuning,
webroot layout, dynamic design/plugin assembly, PHP extension import, artifact
import, and missing-twin findings.

- Docker requests `STOPSIGNAL SIGINT`; Cix now has the directive, so regenerate this case with that declaration. → stale ([STOPSIGNAL disposition](../../../../cips/dispositions.md#batch-2026-08-04-blessed-by-mathijs-dockermd-application-queued))
- Both source FETCH work trees use TOFU consumed pins rather than `EXPECT`; their published SHA-256 checks remain mandatory. The 2026-08-05 update probes each read identical outputs twice and the supplied login probe passed, but cold replay still exposes the independent `designs` warm/cold read-set divergence. → language (cold divergence audit)
