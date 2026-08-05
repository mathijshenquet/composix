# CIP-109: probe-url — URL-shaped probe targets, path-only sugar (CIP-light)

Status: **accepted** (2026-08-05; CIP-light; amends CIP-79).

**Problem (measured).** Two independent cold agents (terra on an
earlier wave, luna on wallos today) tripped over the probe grammar
identically, needing three attempts to reach
`READINESS http 127.0.0.1:80/health.php IN 20s`: first the natural
`http /health.php` (rejected with the misleading `probe target ""
must be host:port`), then `http 127.0.0.1:80 /health.php` (path must
be glued). The Docker intuition is "a path on my own service" —
CIP-79 itself records that health commands are nearly always
`curl -f http://localhost/…`. The explicit host:port is ceremony in
the standard case, and the corpus keeps paying it. Wallos's staged NOTES
also records the same friction while translating Docker's `/health.php`
check into native readiness/liveness; its final spelling remains the
awkward `http 127.0.0.1:18092/health.php` form.

**Proposal (v2 — Mathijs: even the explicit form fights intuition;
people write URLs).**
1. **URL form is the canonical explicit spelling**: the scheme carries
   the probe kind — `READINESS http://127.0.0.1/health.php IN 20s`,
   `LIVENESS tcp://127.0.0.1:5432 EVERY 2m`; `notify` stays a bare
   keyword. Standard URL semantics apply, so `:80` is implicit for
   http — the whole `http 127.0.0.1:80/health.php` token-dance
   disappears. The old two-token form is rejected with a rewrite hint.
2. **Path-only sugar** on top: `READINESS /health.php IN 20s` is valid
   when the service declares exactly one PORT — resolves to
   `http://127.0.0.1:<that port><path>`. Zero or multiple ports → the
   error lists each declared port with its full-URL spelling.
3. Diagnostics teach in both directions: the current
   `probe target "" must be host:port` class is replaced by messages
   that show the resolved URL to write.

**Effort.** S/M: URL parsing for probe targets + sugar resolution +
diagnostics + parser tests + mechanical corpus/examples sweep of
existing probe lines; no unit-generation change (resolution at
compile).

## Decision

Adopted 2026-08-05 (Mathijs: "ja go"). URL form canonical (scheme =
probe kind, standard URL port defaulting), path-only sugar for
single-port services, teaching diagnostics both ways, old two-token
form rejected with a rewrite hint. Implementation queued (S/M incl.
the corpus/examples probe-line sweep).

Changelog:
- 2026-08-05 — adopted as CIP-109.
