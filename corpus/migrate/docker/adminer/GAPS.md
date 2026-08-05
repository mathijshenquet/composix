Generated: CIP-102 volatile-fetch sweep · 2026-08-05
Status: current

This regeneration resolves the earlier version/checksum binder, PHP tuning,
webroot layout, dynamic design/plugin assembly, PHP extension import, artifact
import, and missing-twin findings.

- `ADMINER_DESIGN` and `ADMINER_PLUGINS` use the private `__cix_unset__` sentinel because Cix cannot declare an optional runtime `ENV` with no default. → language (optional ENV declaration)
- Docker requests `STOPSIGNAL SIGINT`; Cix has no service stop-signal directive, so systemd's normal termination semantics remain authoritative. → language ([recorded stop-signal gap](../../../../cips/open-questions.md#proposed-one-line-dispositions-awaiting-mathijs-batch-blessable))
- Both source FETCH work trees use TOFU consumed pins rather than `EXPECT`; their published SHA-256 checks remain mandatory. The 2026-08-05 update probes each read identical outputs twice and the supplied login probe passed, but cold replay still exposes the independent `designs` warm/cold read-set divergence. → language (cold divergence audit)
