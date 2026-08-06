# track/narhash-regens — shrink the legacy fetch-level narHash inventory (CIP-107 pin leg)

Context: CIP-107's FetchPin-deletion leg is blocked on legacy
whole-tree `narHash` entries in corpus locks. There are TWO narHash
populations — read `crates/cix-build/src/lock.rs` first:
- `InputLock.narHash` (nixpkgs inputs): legitimate, OUT OF SCOPE.
- Fetch-level `FetchPin.narHash`: two kinds — author-declared EXPECT
  whole-tree assertions (KEEP; they are the EXPECT mechanism) and
  LEGACY AUTOMATIC pins retaining their former whole-tree value until
  refreshed (the prune target).

Do:
1. **Precise inventory first**: for every
   `corpus/migrate/docker/*/Cixfile*.lock`, list fetch-level narHash
   entries and classify each as EXPECT-backed (the Cixfile carries an
   EXPECT for that FETCH) or legacy-automatic. Record the table in
   your LOG entry before touching anything.
2. **Refresh the legacy-automatic cases, NON-pnpm only**: regenerate
   per each case's receipt.md documented command so refreshed locks
   drop the legacy value. OUT OF SCOPE: dozzle, verdaccio, directus,
   it-tools, and any other pnpm-ecosystem case (in flight on
   track/pnpm-frozenstore).
3. **Known upstream-drift walls** (today's receipts: redis, memcached,
   haproxy, mosquitto — EXPECT drift on tarballs/GPG keys): expect
   warm regeneration to fail there; record the wall, and NEVER update
   EXPECT values on your own authority (that is a translation change,
   not a refresh).
4. Close with a whole-corpus lock hash diff proving only refreshed
   cases' locks changed; corpus browser regenerated; corpus suite
   green. Report the remaining legacy inventory (count + where) at the
   end of your LOG entry — that number is what CIP-107 needs.

Gates: fmt + corpus suite; no Rust changes expected.

Discipline: branch `track/narhash-regens`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Synchronous value-checked
receipts only; walls are valid outcomes. Clean committed branch; do
not merge.
