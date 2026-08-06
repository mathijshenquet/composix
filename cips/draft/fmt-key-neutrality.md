# fmt-key-neutrality — formatting must not invalidate build inputs (CIP-light)

Status: **draft** (2026-08-05), with current-behavior evidence added
2026-08-06.

## Problem

D59 deliberately makes declared builder `ENV` text part of the resolved step
chain, while CIP-87 makes a FETCH replay depend on that resolved step identity
and its captured input snapshot. That is correct for semantic text, but a
formatter's whitespace-only indentation cannot become semantic input. Today it
does: the HAProxy Cixfile is accepted unindented and locked; `cix fmt` indents
the directives, and the same FETCH gets a new identity.

Exact independent reproduction on 2026-08-05:

```text
$ target/debug/cix build /var/tmp/composix-haproxy-fmt.FwbaR9#haproxy
/nix/store/h0n4r9i2c13sciwmv7w26qfzbzgbfj96-cix-item-haproxy

$ target/debug/cix fmt /var/tmp/composix-haproxy-fmt.FwbaR9

$ target/debug/cix build --cold /var/tmp/composix-haproxy-fmt.FwbaR9#haproxy
Error: FETCH builder:build:3-00ba784e8f7b has no locally cached replay snapshot at
/home/mathijs/.cache/cix/fetch-snapshots/3378f6418827b7c769e19aefc1f52f90dce578bebce743ab98056cd4c5e2336d;
run a non-cold build first (--cold never refetches)
```

The unformatted build and the formatted-copy formatter both exited 0; the
formatted cold build exited 1. The only intended change was indentation.

## Evidence — current keying behavior (2026-08-06)

This is a characterization of the implementation that exists today, not an
adoption proposal. A NAR identity for the ordinary filesystem objects in scope
is: file type plus bytes plus the executable bit; symlink type plus target;
directory type plus sorted children. Device/inode, timestamps, ownership, file
size (which follows from bytes), and non-executable permission bits are not
part of that identity.

The three green corpus observations remain useful symptoms, but their original
shared explanation does not survive the code trace:

- **ntfy.** `dev`, `inode`, and `mtimeNs` are serialized beneath
  `stepMemo.*.reads.*.fingerprint`. They are a fast-path validation hint:
  equality of `ReadDependency::File` compares only `hash`, and a mismatch
  merely makes the validator recompute that hash. `build_fingerprint` omits
  both `stepMemo` and `memo`. Those fields therefore cannot cause the recorded
  output `sourceHash` to move. The receipt proved an output-lock change, but
  not the proposed dev/inode-to-sourceHash causal chain.
- **nginx.** Its recorded lock has only `inputs` and `outputs`; it has neither
  a FETCH pin nor a step memo. The old/new `sourceHash` pair consequently
  cannot have flowed through either full-mode read hashing or the persisted
  fingerprint fields. At the point of observation, the receipt did not retain
  a content manifest of the untracked `context/` tree or the exact cix binary,
  so no single changed `build_fingerprint` input can be named retrospectively.
- **tomcat.** The same lock shape rules out a traced-read/FETCH-pin causal
  chain for its `sourceHash`. Its changed `storePath` is likewise not evidence
  that `sourceHash` itself keys the Nix expression: the build path records the
  hash after realization, while code generation and realization have separate
  inputs. The receipt did not retain a derivation/input diff, so its immediate
  cause is unproven.

That limit is itself a result: a green output whose only retained evidence is a
post-hoc lock diff is insufficient to attribute a key leak. Future exhibits
need the pre/post complete source-tree manifest, selected Cixfile, serialized
lock fields that feed the key, `BUILDER_FINGERPRINT`, generated expression, and
the value-checked command output.

| Site and current serialization | Fields used today | Key or decision reached | Exhibit class it does / does not explain | NAR-invariant replacement |
| --- | --- | --- | --- | --- |
| `cix-cixfile::build::hash_source_tree` | Relative path, type tag, bytes (with the selected Cixfile formatted), symlink target; every non-lock, non-`.git` source-tree file is included | `source_tree_hash` → `build_fingerprint` → output `sourceHash` cache check | Can explain a sourceHash move when any included source byte moves, including a receipt or an untracked corpus `context/`; deliberately cannot explain dev/inode/mtime churn. It currently **misses executable-bit changes**. | Traverse the declared build context, encode type/content/executable bit and symlink target only; exclude non-input documentation and unrelated source-tree material. |
| `build_fingerprint` | `BUILDER_FINGERPRINT`, source-tree digest, JSON serialization of `inputs`, `artifacts`, `fetches`, and `dev_envs`; explicitly not `memo`, `stepMemo`, `evalPlan`, or `outputs` | Output cache `sourceHash` | Establishes the closed set for all three sourceHash observations. The focused test proves that a `stepMemo` does not change it and a FETCH pin does. | Serialize canonical semantic inputs only, with every filesystem digest supplied by the NAR-invariant primitive. |
| `trace::read_hash` | Full `st_mode` bytes, then file bytes; for symlinks, target and (when a file) dereferenced bytes | `ReadDependency::File.hash`; subtree/directory hashes incorporate it; read-set equality controls memo reuse | Explains a traced read invalidating on `0644 → 0600` despite equal NAR meaning. It does not feed output `sourceHash` directly. | Hash object kind + bytes + executable bit; preserve symlink target without dereferencing it as identity. |
| `trace::file_fingerprint` | `dev`, `inode`, `mtimeNs`, `size`, `len`, full `mode` | Serialized `ReadDependency::File.fingerprint`; validator fast path only | Explains rehash work after copy/rename or timestamp movement, not a semantic key change and not any recorded sourceHash change. | Keep only as an unkeyed optimization hint, or remove it; correctness comparison remains the NAR-invariant dependency hash. |
| `trace::directory_hash` and `filesystem_subtree_hash` | Sorted child names/kinds and each child hash; child file hash currently has full mode | `ReadDependency::Directory` / `Subtree` read-set keys | Extends the `read_hash` non-executable-mode leak to a complete observed directory. | Use the same NAR-invariant child encoding. |
| `fetch_state::file_fingerprint` | Full permission mode plus file bytes, or symlink target | Automatic `FetchPin.paths` value | Explains automatic FETCH pins moving on `0644 → 0600`; it is separate from `trace::file_fingerprint` and has no dev/inode/mtime fields. | Hash type/content/executable bit/symlink target only. |
| `FetchPin::key` and lock serialization | Explicit `narHash`, otherwise JSON of automatic `paths`; `snapshotNarHash` and volatility facts serialize in the lock but are not in `FetchPin::key` | FETCH snapshot-cache key and `MemoEngine` FETCH/BUILDER step keys; the whole `fetches` map also enters `build_fingerprint` | This is the proven bridge from an automatic full-mode path hash to both step keys and output `sourceHash`. Neither nginx nor tomcat had a FETCH pin, so it cannot explain their recorded pairs. | Retain explicit NAR pins; make automatic path values NAR-invariant before serializing/keying them. |
| `workspace::nar_hash` for declared COPY and consumed paths | `nix hash path --mode nar` | Builder COPY step keys and consumed-output identities | Already uses NAR semantics; not an exhibit source. | No change in semantics. |
| `OutputReceipt` | Prior `sourceHash` and `storePath` | Output-cache eligibility only; omitted from the next `build_fingerprint` | Records all three symptoms but creates no causal input edge. | Keep it a receipt, never an input. |

Hermetic regression/characterization coverage now makes the non-NAR behavior
auditable:

- `current_read_hash_characterizes_full_posix_mode_keying` proves equal bytes
  produce distinct traced-read hashes for both a non-executable permission-only
  change (`0644 → 0600`) and an executable-bit change (`0644 → 0755`).
- `automatic_fetch_path_fingerprint_characterizes_full_posix_mode_keying`
  proves the equivalent property for automatic FETCH path values.
- `build_fingerprint_characterizes_fetch_pin_and_source_tree_inputs` proves
  that arbitrary source-tree content and a FETCH pin change `sourceHash`, while
  a regular file's non-executable mode and an entire `stepMemo` do not.

All three are explicitly tests of **CURRENT** behavior. They pin the evidence
for a later repair; they do not bless full-mode keying.

## Proposal

Compute every semantic key from the parser's canonical form (or an equivalent
canonical AST serialization), never from raw declared text. Preserve D59's
meaning: changing an ENV name, value, directive order, imports, command, or
resolved arguments still changes the key. Normalize only syntax that `cix fmt`
is allowed to rewrite: indentation, whitespace between tokens, and canonical
quote/layout representation where the parsed value is identical.

Add a regression fixture that locks and cold-replays a deliberately
non-canonical Cixfile, formats a copy, and asserts identical FETCH identities,
the same snapshot lookup, and the same item output. This is the source-level
dual of CIP-87's constructive-trace invariant: representational changes must
not cause an invented input change.

## Effort

M. Centralize key serialization at the resolved AST boundary, version/fingerprint
the old raw-text identities honestly, and add warm/cold formatter-equivalence
coverage.
