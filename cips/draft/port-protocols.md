# port-protocols — protocol-aware inbound port declarations (UDP first)

Status: **draft** (2026-08-04, promoted from the corpusgaps sweep: caddy's
GAPS.md, loop-1).

## 1. The problem

`PORT name = 443` declares a TCP contract. Caddy's upstream image exposes
`443/udp` for HTTP/3 (QUIC); the corpus conversion cannot declare it, so the
sandbox-authorization story (D24 `SocketBindAllow=` compiled from declared
ports) silently has no UDP row and the migration carries an undeclarable
loss. Any QUIC/DNS/syslog/WireGuard-shaped workload hits the same wall.

## 2. Prior work

Docker's `EXPOSE 443/udp` and compose `ports: - "443:443/udp"` make the
protocol part of the port grammar. systemd's `SocketBindAllow=` takes
`[address-family:][protocol:]port` — the enforcement layer already speaks
protocol. Composix D29 gave `PORT` its enforced-capability meaning and D24
compiled bind allow/deny lists; both were TCP-shaped because nothing in the
corpus had demanded otherwise. Caddy now does.

## 3. Recommendation

Extend the grammar with an optional trailing protocol word, default `tcp`:

```dockerfile
PORT http = 443
PORT http3 = 443 udp
```

Compile it through to `SocketBindAllow=udp:443` (and the deny mirror), the
manifest, `cix inspect`, and compose publish wiring. Named listeners
(`LISTENER`) stay TCP/stream until a real datagram-activation case appears —
fd-passed UDP sockets are a genuinely different contract
(`ListenDatagram=`), and speculative support is complexity without a
consumer.

## 4. Open questions

- Spelling: `PORT http3 = 443 udp` (word, matches `required`'s position in
  ENV) vs `PORT http3 = 443/udp` (Docker muscle memory)?
- Does compose `publish` need protocol plumbing in the same leg, or is
  declare+bind-allow the honest v0?
- SCTP: refuse until asked, or reserve the grammar now?
