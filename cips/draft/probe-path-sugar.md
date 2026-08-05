# probe-path-sugar — path-only READINESS/LIVENESS targets (CIP-light)

Status: **draft, CIP-light** (2026-08-05; CIP-79 amendment proposal).

**Problem (measured).** Two independent cold agents (terra on an
earlier wave, luna on wallos today) tripped over the probe grammar
identically, needing three attempts to reach
`READINESS http 127.0.0.1:80/health.php IN 20s`: first the natural
`http /health.php` (rejected with the misleading `probe target ""
must be host:port`), then `http 127.0.0.1:80 /health.php` (path must
be glued). The Docker intuition is "a path on my own service" —
CIP-79 itself records that health commands are nearly always
`curl -f http://localhost/…`. The explicit host:port is ceremony in
the standard case, and the corpus keeps paying it.

**Proposal.**
1. Path-only form: `READINESS http /health.php IN 20s` (and LIVENESS)
   is valid when the service declares **exactly one** PORT: it
   resolves to `127.0.0.1:<that port><path>`. With zero or multiple
   declared ports the explicit form stays required, and the error
   lists the declared port names with the resolved spelling for each.
2. The first-attempt diagnostic improves regardless: instead of
   `probe target "" must be host:port`, teach — "path-only probes
   need exactly one declared PORT (found: http=80, metrics=9090);
   write `http 127.0.0.1:9090/health` or reduce to one port".
3. Explicit form remains canonical in generated/migrated output? No —
   fmt keeps whichever the author wrote; the sugar is a real form,
   not an input-only alias (it reads better in the common case).

**Effort.** S: one resolution rule in probe parsing + two diagnostics
+ a parser test; no unit-generation change (resolution happens at
compile).
