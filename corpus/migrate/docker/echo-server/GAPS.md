Generated: CIP-102 volatile-fetch sweep · 2026-08-05
Status: current

- `install-dependencies.sh` is conversion-owned build machinery used to isolate npm's cache and logs; it is not part of the upstream deploy unit. → case
- The moving `node:lts-alpine` image is represented by locked nixpkgs Node rather than an exact Node/Alpine image identity. → case
- The script-driven dependency FETCH uses a TOFU consumed pin rather than `EXPECT`. Its 2026-08-05 update probe read identical outputs twice, the supplied HTTP probe passed, and pinned cold replay rebuilt the same item. → evidence
