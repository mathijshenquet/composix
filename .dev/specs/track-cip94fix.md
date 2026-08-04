# track/cip94fix — remove CIP-94's nested userns requirement

Main CI fails while realizing `buildCixfile`: the eval-from-lock FETCH/RUN
derivations invoke bubblewrap inside Nix's build sandbox, and the runner denies
unprivileged user namespaces. Preserve byte identity with `cix build --cold`
wherever the derivation builds.

Evidence order:

1. Reproduce the denial with the byte-identity check under a guest/Nix build
   configuration that disables unprivileged user namespaces.
2. Prefer Nix's existing build isolation for FETCH FODs and ordinary RUN
   derivations; retain only a userns-free emulation of the cix filesystem
   skeleton if byte identity requires it.
3. If exact replay genuinely cannot work, make the unsupported-host boundary
   a loud eval-time error naming user namespaces and keep the check mandatory
   on capable hosts.
4. Record the evidence-selected boundary in CIP-94 and consumer docs, run the
   standard agent tier plus the focused no-userns byte-identity receipt, and
   commit on `track/cip94fix`.

Touch the CIP-94 Nix library/check, its accepted CIP and consumer documentation,
and the ignored local task journal. Re-grade Docker/corpus ledgers only if their
claimed behavior changes; grep migration GAPS for CIP-94/buildCixfile currency.

Evidence selected options 1 and 2 together: the eval-from-lock derivations use
Nix's existing isolation boundary and a userns-free `proot` filesystem view.
In the focused VM, `user.max_user_namespaces=0` made an unprivileged
`unshare --user` fail; the FETCH FOD and ordinary RUN then realized and their
result matched `cix build --cold` by NAR hash. No unsupported-host escape hatch
is needed.
