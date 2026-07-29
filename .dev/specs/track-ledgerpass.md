# Task: reconcile docs/docker.md against everything that has landed

The ledger's dispositions and "still missing" column predate several merges. Reconcile it —
row by row — against current reality. Do NOT soften judgments; where something is still
missing, it stays. Territory: `docs/docker.md` ONLY. COMMIT AS YOU GO.

Landed since the audit (verify each in the code/design before touching a row):

1. **Cixfile v1 is implemented**: `cix build`, line-numbered parse errors, `Cixfile.lock`
   (nixpkgs rev + narHash, tamper-detected), COPY sibling files, FILE/SCRIPT heredocs, LINK,
   multi-service SERVICE blocks → section 3 rows still saying "unimplemented"/"no parser or
   cix build" are stale.
2. **D24 built**: declared ports compile to `SocketBindAllow/Deny` — kernel-enforced bind
   restrictions, live-tested (EPERM on undeclared bind). Docker has no equivalent; the
   capabilities/ports rows may now honestly claim this as stronger-than-docker WITH the test
   as receipt (cite `crates/cix-run` integration test).
3. **spec v3 `listeners`**: socket-activated services with zero IP-socket authority;
   `cix run -p name=addr` binds via transient socket units (`examples/listenfds`). Touches
   EXPOSE/port rows and the networking "no bind-address control" claim (bind exists for
   listeners; a port *inventory* still does not).
4. **D22 v3 filesystem projection**: items are sparse rootfs fragments projected read-only at
   native absolute paths; stress-tested (host-dir shadowing ro, symlink escape blocked, 25
   mounts). Relevant to `--read-only`/VOLUME/image rows.
5. **Evidence we owe**: mark the transfer-size item as having a first datapoint (~25 MiB
   compressed OCI nginx vs ~65 MiB rootfs item, from the import prototype — nuance: that
   measured an *imported rootfs* item, not a native sparse item, which is kilobytes plus
   shared store closure); mark reproducibility-enforcement as partially delivered
   (Cixfile.lock narHash verification) with what remains (rebuild verification).
6. Do NOT touch section 6 (Compose) — compose v0 is mid-flight in another track; it gets its
   pass after merging.

Gate: every changed row verified against the actual code/design (cite Dnn or a file path in
the row where it strengthens a claim); markdown tables stay 3-column and render; committed;
clean status. Final commit message body: one-paragraph summary of what moved.
