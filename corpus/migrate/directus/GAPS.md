Generated: migrate.md@d582f41 · unknown · 2026-07-31
Status: current

- The Cix build fails when the downloaded Sass executable requests an FHS dynamic loader; the corpus page must say plainly that no item is produced, rather than presenting an unexplained orange state. → browser
- Reproduce and retain the exact loader diagnostic in a fresh build receipt once the pinned source is materialized again. → evidence
- Builders cannot provision or patch a downloaded FHS-linked ELF's interpreter through today's bare `IMPORT` surface. → language (candidate: builder ELF loader support)
- The builder's `mkdir`/`rm`/`ln` dance redirects application-relative state paths, and the service separately links Node into `/bin`; both are symptoms of the missing artifact import/runtime-path link canon. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- Keep the good narrow handoff `COPY ${build}/dist /directus` when regenerating; it copies the deploy unit instead of leaking the whole workshop. → case
- Split at least one of database, extensions, or uploads into a second `STATEDIR` to demonstrate role multiplicity directly, as Wallos already does for database and logos. → case
- `NPM_CONFIG_UPDATE_NOTIFIER=false` disappears without an explicit “runtime package manager dissolved” disposition; parity review requires the disposition even when the environment is intentionally unnecessary. → case
