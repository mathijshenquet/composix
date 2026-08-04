# Chapter 2: The Cixfile language

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will grow the first service into an example of every everyday Cixfile declaration. Afterwards, you will understand the backward-only graph, explicit binders, and capability vocabulary well enough to read a Cixfile without hidden Docker assumptions.

## A graph you can read from top to bottom

A Cixfile is a backward-only graph. `FROM` binds inputs, a block binds the artifact it creates, and `${name}` can refer only to something declared earlier; there are no ambient package names or inherited filesystem layers. The local directory is the one deliberate convenience: a bare `COPY index.html …` is the same source as `${src}/index.html` after `FROM . AS src`.

```sh
$ cat Cixfile index.html service.conf
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE language
IMPORT ${pkgs.coreutils} ${pkgs.busybox} ${pkgs.bash}
COPY index.html /srv/language/index.html
COPY ${pkgs.coreutils}/bin/printf /opt/tools/printf
COPY ${pkgs.nginx}/conf /opt/nginx
COPY service.conf /etc/language/service.conf
FILE /etc/language/build-origin <<ORIGIN
packages=${pkgs.coreutils}
ORIGIN
START sleep 60
ENV SITE_NAME = guide
ENV API_TOKEN required
STATEDIR /var/lib/language
STATEDIR /opt/nginx/state
CACHEDIR /var/cache/language
LOGDIR /var/log/language
CONFIGDIR /etc/language
RUNDIR /run/language
PORT web = 8088
PORT dns = udp:5353
LISTENER admin
CLAIM egress
CLAIM jit
language guide
root=/srv/language
state=/var/lib/language
```

## IMPORT and COPY

`IMPORT` unions each package's `bin`, `etc`, and `share` trees into the item. It accepts ordinary package references, bare commands such as `sleep` resolve through that union, and earlier imports win a collision—so coreutils supplies `ls` even though busybox comes later.

`COPY` makes its storage choice from provenance. Local source bytes are materialized; package, FETCH, builder, and cix-item sources normally remain links into immutable store trees. A later write or a runtime directory mount beneath a link forces that branch to materialize, which is why the package tree containing `STATEDIR /opt/nginx/state` becomes a real directory while `/opt/tools/printf` remains a link.

```sh
$ cix build .
{"language":"/nix/store/…-cix-item-language"}
```

```sh
$ test -f /nix/store/…-cix-item-language/srv/language/index.html && test -L /nix/store/…-cix-item-language/opt/tools/printf && test ! -L /nix/store/…-cix-item-language/opt/nginx && printf 'local: materialized\npackage: linked\nmount ancestor: materialized\n'
local: materialized
package: linked
mount ancestor: materialized
```

`FILE` creates the small interpolated `build-origin` file below. It is useful when the content genuinely needs a binder value; for ordinary configuration it is a smell, because a checked-in file plus `COPY` stays easier to lint, edit, and test.

```sh
$ cat /nix/store/…-cix-item-language/etc/language/build-origin
packages=/nix/store/…-coreutils-9.11
```

## Runtime declarations are grants

`ENV SITE_NAME = guide` supplies a default, while `ENV API_TOKEN required` requires an operator value without baking one into the item. Role directories use the application's native absolute paths and state their lifecycle: state persists, cache is disposable, logs are retained, config is operator-managed content, and run data disappears on stop.

A bare port is TCP; the `udp:` prefix is the single UDP spelling. `LISTENER admin` is different: systemd owns a TCP socket and passes its file descriptor to the service, which is useful for socket activation and privileged binds. Chapter 6 executes that boundary in a compose setting.

`CLAIM egress` admits outbound networking, and `CLAIM jit` allows writable executable memory. Without those explicit declarations, the corresponding sandbox authority stays denied.

```sh
$ jq '{env, ports, listeners, dirs, claims}' /nix/store/…-cix-item-language/cix-manifest.json
{
  "env": {
    "API_TOKEN": {
      "required": true
    },
    "PATH": {
      "default": "bin"
    },
    "SITE_NAME": {
      "default": "guide"
    }
  },
  "ports": {
    "dns": {
      "protocol": "udp",
      "value": 5353
    },
    "web": {
      "protocol": "tcp",
      "value": 8088
    }
  },
  "listeners": {
    "admin": {
      "type": "stream"
    }
  },
  "dirs": {
    "cache": [
      "/var/cache/language"
    ],
    "config": [
      "/etc/language"
    ],
    "logs": [
      "/var/log/language"
    ],
    "run": [
      "/run/language"
    ],
    "state": [
      "/opt/nginx/state",
      "/var/lib/language"
    ]
  },
  "claims": [
    "egress",
    "jit"
  ]
}
```

## Directive reference

| Declaration | What it adds |
| --- | --- |
| `FROM … AS name` | A locked package/source/item binder; `FROM .` names local context. |
| `FETCH name command … EXPECT hash` | A pinned network step and reusable source binder. |
| `BUILDER name` | A persistent build workspace for `COPY`, `ENV`, `FETCH`, and offline `RUN`. |
| `SERVICE name` / `APP name` / `ITEM name` | A long-running runtime contract / run-to-completion contract / manifest-less tree. |
| `IMPORT package…` | An earlier-wins read-only package union with bare command lookup. |
| `COPY source /destination` | Provenance-aware item assembly; builder destinations are workdir-relative. |
| `FILE /destination <<EOF` | An inline interpolated file; prefer checked-in files when possible. |
| `START` / `START_PRE` | The argv entrypoint / idempotent service pre-start argv. |
| `ENV` / `SECRET` | Declared runtime configuration / credential-file need. |
| `PORT` / `LISTENER` | A direct TCP/UDP bind / systemd-owned TCP socket. |
| role dirs / `DIR` | Cix-managed lifecycle storage / operator-supplied data. |
| `READINESS` / `LIVENESS` | Startup gating / watchdog restart probes. |
| `CLAIM` / `SHM` | A narrow sandbox exception / bounded private shared memory. |


---

[← Previous](01-hello-composix.html) · [Tour index](index.html) · [Next →](03-build-run-debug.html)
