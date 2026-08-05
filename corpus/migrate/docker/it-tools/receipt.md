# it-tools migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh it-tools`
restored the pinned `d505e5f` source context.

Before the final service correction,
`target/debug/cix build --update-lock build .#web` exited 0 and produced
`/nix/store/11a2c3n99ba3ny0np6nwkhrsnvyllg89-cix-item-web`; pnpm fetch and the
offline Vue/Vite build completed. Its system-manager run then failed because
nginx could not open `/var/log/nginx/{access,error}.log` under DynamicUser.

After removing that false `LOGDIR`, a fresh `timeout 600 target/debug/cix build
--update-lock build .#web` observed the volatile pnpm store/index probe and was
still alive after the 600-second bound: its timeout wrapper did not reap the
child, so a manual interrupt ended the supervisor with status 130 before it
emitted an item. No corrected runtime success is claimed.
