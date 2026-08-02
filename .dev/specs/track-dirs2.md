# track/dirs2 — CIP-82 leg 2: compose materializations, clean/purge

Read AGENTS.md first. Authoritative: cips/accepted/0082-dirs.md — the
Materialization table, "Ownership at the host seam", the Lifecycle
table (normative), and §5 Decision. Leg 1 (overlay backing, arbitrary
paths, `DIR`, unit-scoped mirrors) landed; this leg is the compose
side plus the lifecycle verbs. Work in `.worktrees/dirs2` on branch
`track/dirs2`. Keep `crates/cix-compose/LOG.md` current. SEAM NOTE:
track/ergo runs concurrently in crates/cix-cixfile (FROM attrs, lock
env) — this track needs NO Cixfile parser changes (all vocabulary is
compose-side); stay out of that crate's parser/builder.

1. **Compose materialization vocabulary** (per declared dir, per the
   CIP table):
   - default private: already leg 1 — untouched.
   - `host: /path` → `BindPaths=`/`BindReadOnlyPaths=` per role
     write-ness + `RequiresMountsFor=`; pre-existence checked at up
     (loud error naming the path); **static identity (D48d) required**
     — dynamic-identity members with host backing are a check error;
     cix never chowns and never mkdirs outside its own roots.
   - `shared: <name>` → composite-owned surface, v0 legal on STATEDIR
     and DIR only; stable registry group + setgid dir + supplementary
     group membership + `UMask=0002`; hermetic: every member using the
     name must have declared the dir and roles must agree — else check
     error.
   - `as: <role>` → treatment reclassification; escalate (cache→state)
     silent, degrade (state→cache) LOUD in `compose check` (D49a
     polarity).
   - Extra operator binds of undeclared paths: possible, ro unless
     `write: true`, always loud in check.
   - Idmapped bind onto foreign-owned host data: requires the explicit
     acknowledgment field (§5 ruling: silent identity-mapping is
     refused). Pick a clear field spelling, record the choice in the
     LOG, and refuse idmapping without it.
2. **`.env` containment**: interpolation resolves from the compose
   file's own directory `.env` only; resolved values enter the
   generation identity (changed `.env` ⇒ restart-changed sees it);
   refuse any secret-shaped delivery road (CIP-81 owns credentials).
3. **Lifecycle verbs** (the table is the normative contract):
   - `cix clean --what=<class>`: CACHEDIR removable, LOGDIR opt-in,
     STATEDIR/DIR/shared refused (exact refusal wording naming the
     remedy), host-backed untouched.
   - composite removal `--purge`: removes private+shared roles,
     NEVER DIR or host-backed; interactive confirmation printing the
     exact paths, `--yes` for automation.
   - `cix recreate` refused with the migration message (`cix up`
     converges; expendable state is `cix clean`). No implicit-deletion
     verbs anywhere.
4. **`cix run` parity** (CIP-77): the same materialization flags on
   the run surface.
5. **Ledgers** (per the standing convention): docker.md named-volume /
   bind-mount / volume-CLI rows move from queued to honest ✅/🔶;
   docs/corpus.md rows blocked on operator-binds and shared-rw
   (Compose 2, 7, 8, 9, 10, 14; the open-gaps table rows 2 and 3)
   re-graded — receipts only where you actually ran the case.
6. **Tests**: compose schema accept/reject matrix (all vocabulary ×
   role legality, degrade-loud, hermetic-shared violations, missing
   idmap acknowledgment); unit-gen fixtures for host/shared/as; new
   `nix/scenarios/dirs2.nix`: a pre-existing host dir with static
   identity binds and survives purge; a shared STATEDIR between two
   members proves group/setgid/umask write-both-sides; clean removes
   CACHEDIR and refuses STATEDIR; purge prints exact paths and removes
   private roles while leaving host/DIR untouched; degrade
   reclassification is loud in check.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.
