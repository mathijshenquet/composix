Generated: migrate.md@666cf74 · unknown · 2026-08-02
Status: stale — regenerate with CIP-91

- The nginx/PHP-FPM configuration pair remains inline only because it needs package interpolation; move it to real files when the pending syntax lands. → language ([FILE … FROM draft](../../../cips/draft/file-from.md))
- `mime.types` and `fastcgi_params` are immutable package assets embedded as store paths inside the nginx heredoc; place those files with `LINK` and keep the authored config package-agnostic. → case
- Six package binaries are linked individually into `/bin`, the corpus's clearest artifact toolset pile. → language ([CIP-91](../../../cips/accepted/0091-artifact-import.md))
- The wrapper still supervises nginx, PHP-FPM, and supercronic inside one service; split them into compose members or state why their coordination requires one unit. → case
- Docker's `/health.php` check passes only in `check.sh`; add native HTTP `READINESS` now that CIP-79 is built. → case
- The historical runtime probe has not been reproduced from source in the closed-root audit. → evidence
