# Compose v1 networking — netns realization proposal

Status: **decided** — Mathijs's read 2026-07-30 closed the open decisions as **D49**
(egress = leaf property + loud compose override; proxyd-only publish; fixed cix-owned
subnet with persisted per-composite IPAM; naming kept). Together with D43 (pod-ness as
scoped property, `network: host` escape dropped) and D48b (the word is `egress`), the
mechanics below are the realization plan, applied at pod-claiming nodes. design.md
wins on any remaining conflict.

> **Amended by the 2026-07-30 tree round (D42–D44, docs/compose-tree.md):** egress is
> a per-service **`egress`** field (opt-in capability polarity kept — the
> zero-machinery-by-default property depends on it); the composite-level `network: host`
> escape dies in favor of *pod-ness as an optional scoped property* (`network: "pod"`,
> nearest-pod-ancestor; absence anywhere = host networking); the "one netns per
> composite" framing generalizes to "one netns per pod-claiming composite". The
> mechanics below (netns lifecycle units, attachment, fd-first publishing, veth/
> masquerade egress tier, IPAM constraints) are unchanged and remain the realization
> plan, applied at pod-claiming nodes.

## Scope claimed for v1

One thing, done completely: **the composite netns of D23**, with explicit host-edge
publishing. Named multi-network objects (D26) and `talks-to` (D27) are deliberately v1.5+;
what v1 must do is not paint them into a corner (constraints listed at the end).

## Mechanics (all existing systemd machinery, no new daemons)

1. **Netns lifecycle**: per composite, a `cix-<comp>-netns.service` oneshot creates a named
   namespace at `/run/netns/cix-<comp>` (loopback up), owned by the composite target;
   `StopWhenUnneeded=` plus teardown on `cix down`. Generated like every other unit in the
   composite's store item — profiles/rollback apply unchanged.
2. **Service attachment**: every service unit gets `NetworkNamespacePath=/run/netns/cix-<comp>`.
   Services inside share one loopback: intra-composite addressing is `localhost:<port>`,
   per-composite `127.0.0.1:5432` collision-freedom exactly as D23 states. D24's
   `SocketBindAllow=` keeps applying verbatim inside the namespace.
3. **Publishing = fd passing first** (the D25 capability tier, which crosses namespaces for
   free): a published port is a host-side `.socket` unit whose fd is handed to the service
   inside the netns — sockets are netns-transparent once passed. For apps that cannot
   accept fds, the fallback pair is host `.socket` + `systemd-socket-proxyd` running inside
   the composite netns (`JoinsNamespaceOf=`/same `NetworkNamespacePath=`), forwarding to
   `localhost:<port>`. nftables DNAT is the last resort and v1 may simply not ship it.
4. **Egress**: a composite whose services make outbound connections gets one veth pair into
   a host bridge with `IPMasquerade=` (networkd-managed) — the "default network" degenerate
   case of D26, without the named-object surface. A composite with no egress need gets pure
   loopback: no veth, no routes, nothing to firewall. Compose v0 compatibility: a
   composite-level `network: host` escape keeps today's behavior, loudly.

## The one genuinely new modeling question: egress is currently unexpressed

The spec models ingress (ports, listeners) but nothing says "this app initiates outbound
connections." Under D20a/b the knowledge is app-semantic (the app either calls external
APIs or does not), which argues for a spec field (v4): `egress: true` — absence means the
service gets a loopback-only view even when the composite has a veth (enforced per-service
via `IPAddressDeny=any` + allow loopback+composite subnet, the D26 mechanism used in its
degenerate form). Alternative: composite-level egress in compose (operator decision,
coarser but no spec bump). This is the main taste call of the round.

## Decisions needed from the round

- **Egress declaration**: spec `egress: true` (app semantics, per-service enforcement) vs
  compose-level (operator, coarse). Proposal leans spec-field.
- **Publish fallback**: is proxyd-in-netns enough for v1, or is DNAT required? Proposal:
  proxyd only; DNAT is an optimization with firewall-interaction baggage.
- **Default-network subnet**: fixed cix-owned range with per-composite allocation persisted
  in composite state (pre-commits us to D26's stable-IPAM constraint) — or link-local
  trickery (rejected by D26's multi-host constraints)?
- **Naming**: `cix-<comp>-netns.service` + `/run/netns/cix-<comp>` — bikeshed now, rename
  never.

## Constraints honored for later phases (so v1 doesn't corner us)

- Stable addressing: any address v1 hands out comes from persisted IPAM state in the
  composite's state dir (survives rollback; rollback restores units, not leases).
- No link-local/broadcast reliance anywhere (D26 multi-host constraint).
- Enforcement stays address-keyed (`IPAddressAllow/Deny`, later `NFTSet=`) so D27's
  `talks-to` compiles onto v1's plumbing without re-plumbing.
- The capability tier remains preferred: everything fd-shaped (listeners, unix edges)
  needs zero of this machinery and keeps working unchanged — netns only governs the
  services that insist on an IP stack.

## Non-goals (v1)

Named multi-networks and cross-composite networks (D26 proper) · `talks-to` (D27) ·
multi-host realization · per-service DNS (D23 made it unnecessary) · IPv6 story beyond
"loopback works" (explicitly revisit at D26).
