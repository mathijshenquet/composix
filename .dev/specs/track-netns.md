# track/netns — CIP-86: the pod netns realization (D43/D49)

Read AGENTS.md first (focused agent gate; synchronous receipts).
Authoritative: cips/accepted/0086-netns.md (the realization plan,
verbatim mechanics) with D43 (pod-ness as scoped property,
nearest-pod-ancestor) and D49 (egress polarity, proxyd-only publish,
fixed cix-owned subnet + persisted IPAM, naming kept) in
docs/design.md as decision records. tree1 just landed the group-node
grammar and REJECTS `network:` — this track un-rejects it and builds
the realization. Work in
`/home/mathijs/worktrees/composix/track-netns` (herdr worktree) on
branch `track/netns`. Keep `crates/cix-compose/LOG.md` current.
Nothing else is in flight.

1. **`network: "pod"`** accepted on any group-node (tree1's schema
   rejection replaced); nearest-pod-ancestor decides each service's
   namespace; absence anywhere = host networking exactly as today
   (zero machinery for pure composites — prove no unit-text diff for
   a podless tree).
2. **Netns lifecycle** (CIP-86 §mechanics): per pod-claiming node a
   `cix-<path>-netns.service` oneshot creating a named namespace at
   `/run/netns/cix-<path>` (loopback up), owned by the composite
   target; teardown on `cix down`; generated like every other unit in
   the store item — profiles/rollback apply unchanged.
3. **Attachment**: member units get
   `NetworkNamespacePath=/run/netns/cix-<path>`. D24 SocketBindAllow
   applies verbatim inside. Intra-pod addressing = localhost:<port>;
   prove per-pod 127.0.0.1 collision-freedom (two pods, same port) in
   the VM.
4. **Publish = fd first**: a published port is a host-side `.socket`
   whose fd is handed into the netns (sockets are netns-transparent
   once passed). Fallback for non-fd apps: host `.socket` +
   `systemd-socket-proxyd` INSIDE the pod netns (JoinsNamespaceOf/
   same NetworkNamespacePath) forwarding to localhost. NO DNAT (D49).
5. **Egress**: only services with `CLAIM egress` (built) get it; a
   pod with any egress-claiming member gets ONE veth pair into a
   cix-owned host bridge with `IPMasquerade=` (networkd-managed);
   per-service loopback-only enforcement for non-claiming members via
   `IPAddressDeny=any` + allows (D26 degenerate form). Fixed
   cix-owned subnet; per-pod allocation persisted in composite STATE
   (survives rollback — rollback restores units, not leases; assert
   this). The CIP-84 resolv.conf bind keeps working for egress
   members inside the netns — verify, don't assume.
6. **Constraints honored** (CIP-86 tail): no link-local reliance;
   enforcement stays address-keyed; the fd tier (listeners, unix
   edges) keeps working unchanged across the boundary — prove an edge
   crossing pod↔host in the VM.
7. **Docs/ledgers**: docker.md networking section rows this makes
   honest (bridge/localhost story, `network_mode` rows); corpus rows
   4/9 (segmentation) re-graded honestly — named networks/talks-to
   (D26/D27) remain era-parked, say so; CIP-86 changelog line;
   docs/design.md "Building now" note.
8. **Tests**: schema acceptance + nearest-ancestor resolution units;
   unit-gen snapshots (netns unit, attachment, veth/egress,
   proxyd fallback); new `nix/scenarios/netns.nix`: two pods with
   colliding internal ports both serve; publish via fd reaches a pod
   member from the host; a non-egress member cannot reach out while
   an egress-claiming sibling can; IPAM lease survives
   up→rollback→up.

If host-bridge/networkd mechanics fight the VM harness, an honest
STOP with findings beats a shallow pass (AGENTS.md scope rule).

Gate (agent side): fmt / examples fmt / warning-denied clippy /
workspace tests / tour regen + drift / focused: scenario-netns +
scenario-tree + compose-fallback-vm. Full matrix at the orchestrator
gate. Commit on this branch when green.
