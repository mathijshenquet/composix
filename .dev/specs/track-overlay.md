# track/overlay — D70 overlay universes + wallos rewrite + trailing EXPECT

Read AGENTS.md first (note the new synchronous-receipts convention).
Authoritative: docs/design.md D70 (a)–(d) verbatim; plus one adopted
grammar nit from Mathijs's 2026-08-02 review, item 3 below. Work in `/home/mathijs/worktrees/composix/track-overlay` (herdr worktree) on branch `track/overlay`. Keep
`crates/cix-cixfile/LOG.md` current. The ergo track just landed in this
crate (FROM lock-metadata attributes `${src.rev}`…, vendored dev-env
snapshots in Cixfile.lock, `--stats`) — build on its landed form.
PARALLEL FENCE: track/readset runs concurrently and owns the builder
engine/memo/--stats internals; you own parser/FROM/universe
resolution/lock inputs. Keep lock-schema changes additive; expect a
merge seam.

1. **`FROM <flakeref> OVERLAY <./file.nix>… AS <name>`** (D70a/b):
   evaluate the base with nixpkgs' own overlay mechanism
   (`import <tree> { system; overlays = [ (import ./file.nix) … ]; }`);
   repeatable, order = overlay order. Checked errors: base must accept
   an `overlays` argument (functionArgs-checked, error suggests
   wrapping the base or a full universe tree); each overlay file must
   be a `final: prev:` function to attrset. Overlays cannot reference
   Cixfile binders. Multiple universes side by side stay legal; add the
   documented world-skew hygiene note.
2. **Keying/lock** (D70c): universe identity in chain keys = (base
   pin, ordered overlay file hashes); overlay files are context
   content; one lock — `--update-lock` moves the base pin, overlay
   edits are ordinary source edits. Compose with ergo's landed lock
   shapes additively (vendored env snapshots key on (universe rev,
   import set) — an overlaid universe's identity must include the
   overlay hashes there too, or the env snapshot lies).
3. **Trailing EXPECT** (adopted grammar nit): `FETCH <command…> EXPECT
   <sri-hash>` replaces the leading `EXPECT` form. Parse rule: EXPECT
   counts only as the penultimate token with a final token that parses
   as an SRI hash; a command legitimately ending in that shape gets a
   spanned error. Migration-grade refusal of the old leading form
   naming the rewrite. Key on the PARSED form so the spelling swap
   orphans no memos (D48a precedent) — verify and state this in the
   LOG. fmt canonicalizes to trailing; sweep examples/, corpus
   Cixfiles, docs.
4. **Wallos rewrite** (the D70 forcing example): replace
   `corpus/migrate/wallos/default.nix` with a Cixfile using
   `FROM … OVERLAY ./php.nix AS pkgs` (php.withExtensions in the
   three-line overlay idiom). `./check.sh cix` must pass — synchronous
   receipt in the LOG. Update the wallos receipt.md honestly (escape
   hatch no longer needed for this case; D4 remains for what OVERLAY
   cannot express) and re-grade the corpus row + regenerate the corpus
   browser page (the generator test does this).
5. **Docs**: docs/cixfile.md — OVERLAY under FROM (with the D70d
   boundary: full universe trees remain the org-wide form), trailing
   EXPECT in the FETCH section; docs/nix-build.md quoted Cixfile only
   if it changes; ledger rows touching package customization.
6. **Tests**: parser + fmt round-trips (OVERLAY repetition/order,
   trailing EXPECT incl. the ambiguous-tail error); functionArgs and
   non-function overlay error fixtures; keying tests (overlay edit
   changes chain key; base pin move via --update-lock only); the
   wallos check as the e2e receipt.

Gate: fmt / `cargo run -- fmt --check examples` / warning-denied
clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. All receipts synchronous exit
statuses. Commit on this branch when green.
