Generated: migrate.md@c43ae9b · terra · 2026-07-30
Status: stale — regenerate with CIP-91

- The upstream `ADMINER_VERSION` and checksum variables were dissolved into a literal URL and `EXPECT`; keep version/checksum structure as builder `ENV` binders so a later version bump remains reviewable. → prompt
- The application and its loader files moved from `/var/www/html` to filesystem root without a stated reason. → prompt
- The upstream PHP tuning file is absent, losing `upload_max_filesize`, `post_max_size`, `memory_limit`, `max_execution_time`, and `max_input_vars`. → case
- The login-page probe deliberately omits the entrypoint's dynamic design and plugin assembly from `ADMINER_DESIGN` and `ADMINER_PLUGINS`; that optional contract remains unconverted. → case
- The package binary is exposed through an implicit `/bin` link instead of a declared artifact tool import. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The upstream `STOPSIGNAL SIGINT` has no Cix manifest counterpart, so systemd's default termination semantics silently replace it. → language ([recorded stop-signal gap](../../../docs/open-questions.md#proposed-one-line-dispositions-awaiting-mathijs-batch-blessable))
- nixpkgs packages Adminer, but this source-faithful conversion has no dissolved twin for the required side-by-side comparison. → case: add dissolved twin
