Generated: CIP-102 volatile-fetch sweep · 2026-08-05
Status: current

- The faithful source archive, published SHA-256, detached signature, config helpers, PHP tuning defaults, sessions path, and upstream application layout are now carried, but the exact `bz2`, `gd`, `mysqli`, `opcache`, `zip`, `bcmath`, and `uploadprogress` extension build remains dissolved into a generic packaged PHP runtime. → case
- Apache, `remoteip`, the upstream entrypoint's base64 config replacement and PHP-version hiding, and its `*_FILE` secret bridge are not reproduced by the PHP built-in server wrapper. → case
- The service listens directly on 8080 to satisfy the repository probe, while the Docker container listens on 80 and relies on host publication for 8080. → evidence
- No dissolved twin applies: an attribute-name audit of locked nixpkgs revision `643809054d65fdd466a63e3155b8c498cb483c04` found neither `phpmyadmin`/`phpMyAdmin` nor a phpPackages match. → evidence
- The mirror/GPG FETCH uses a TOFU consumed pin, not `EXPECT`; its vendor SHA-256 and detached GPG signature remain mandatory. The 2026-08-05 update probe read identical outputs twice, but cold replay still exposes the independent `output` warm/cold read-set divergence. → language (cold divergence audit)
