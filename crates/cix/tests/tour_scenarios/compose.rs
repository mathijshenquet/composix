use super::super::*;

pub(crate) fn chapter_compose() -> String {
    let mut doc = Doc::new("compose");

    doc.para("You will connect two independently built services through a real Unix socket, give both services one persistent directory, and compare two resolved systemd generations. A compose edge names a producer directory and the path where each consumer sees that directory; the shared-directory declaration names storage that both units may write. Rootless probes exercise those data surfaces, while the generated diff stops before privileged system activation.");

    doc.para("## Named listeners are systemd sockets");
    listener_fixture(&doc);
    doc.para("`LISTENER http` asks systemd to create one stream socket named `http`; it does not forbid the program from creating unrelated sockets. The checked-in `listenfds.py` program verifies that systemd passed exactly one listener as file descriptor 3. The operator's `-p http=127.0.0.1:8420` binds that named listener to an address before the service starts.");
    let listener_source = ["listener-fixture/Cixfile", "listener-fixture/listenfds.py"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(listener_source.contains("socket.fromfd(3"));
    assert!(listener_source.contains("COPY listenfds.py /bin/listenfds"));
    assert!(listener_source.contains("LISTENER http"));
    let listener_build = doc.sh(
        "listener_item=$(cix build listener-fixture | jq -r '.[\"listener-demo\"]'); printf '%s\\n' \"$listener_item\"",
        true,
    );
    let listener_path = listener_build
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("listener build printed its captured item")
        .to_owned();
    let manifest = doc.show_file(Path::new(&listener_path).join("cix-manifest.json"));
    assert!(manifest.contains("\"listeners\""));
    let listen = next_listen();
    let started = doc.sh_with_env(
        &format!(
            "unit=$(cix run \"$listener_item\" --user -p http={listen} --detach); printf '%s\\n' \"$unit\""
        ),
        &[("listener_item", &listener_path)],
        true,
    );
    let listener_unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-") && line.ends_with(".service"))
        .expect("cix run printed a listener unit")
        .to_owned();
    let listener_socket = listener_unit
        .strip_suffix(".service")
        .expect("listener unit has a service suffix")
        .to_owned()
        + "-http.socket";
    wait_for_http(&listen, "LISTEN_FDS=1; no socket() authority");
    let response = doc.sh(&format!("curl -fsS http://{listen}"), true);
    assert_eq!(response.trim(), "LISTEN_FDS=1; no socket() authority");
    let stop_listener = format!(
        "socket=${{unit%.service}}-http.socket; {}; {}",
        idempotent_user_stop_command("$socket"),
        idempotent_user_stop_command("$unit")
    );
    doc.sh_with_env(&stop_listener, &[("unit", &listener_unit)], true);
    wait_for_user_units_gone([listener_unit.as_str(), listener_socket.as_str()])
        .expect("listener receipt unloads after stop");
    stop_empty_cix_run_slice("the compose listener receipt");

    doc.para("## Connect two real item programs");
    for name in ["producer", "consumer"] {
        fs::create_dir(doc.base.join(name)).expect("creating compose member directory");
    }
    fs::write(
        doc.base.join("producer/producer.py"),
        r#"#!/usr/bin/env python3
import os
import socket
import sys
from pathlib import Path

path = Path(sys.argv[1])
version = sys.argv[2]
path.parent.mkdir(parents=True, exist_ok=True)
if path.exists():
    path.unlink()
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(str(path))
server.listen()
print(f"producer {version} listening at {path}", flush=True)
while True:
    connection, _ = server.accept()
    with connection:
        request = connection.recv(4096)
        connection.sendall(f"producer {version} received ".encode() + request)
"#,
    )
    .expect("writing Unix producer probe");
    fs::write(
        doc.base.join("consumer/consumer.py"),
        r#"#!/usr/bin/env python3
import socket
import sys
import time

path = sys.argv[1]
for attempt in range(100):
    try:
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        client.close()
        time.sleep(0.05)
else:
    raise SystemExit(f"producer socket never became ready: {path}")
with client:
    client.sendall(b"ping")
    print(f"consumer connected to {path}: {client.recv(4096).decode()}")
"#,
    )
    .expect("writing Unix consumer probe");
    for path in ["producer/producer.py", "consumer/consumer.py"] {
        use std::os::unix::fs::PermissionsExt;
        let path = doc.base.join(path);
        let mut permissions = fs::metadata(&path)
            .expect("reading Unix probe permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("making Unix probe executable");
    }
    fs::write(
        doc.base.join("producer/Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE producer-v1
IMPORT ${pkgs.coreutils} ${pkgs.python3}
COPY producer.py /bin/producer-probe
START producer-probe /run/producer/service.sock v1
RUNDIR /run/producer
STATEDIR /var/lib/shared
"#,
    )
    .expect("writing producer Cixfile");
    fs::write(
        doc.base.join("consumer/Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE consumer
IMPORT ${pkgs.coreutils} ${pkgs.python3}
COPY consumer.py /bin/consumer-probe
START consumer-probe /run/upstream/service.sock
STATEDIR /var/lib/shared
"#,
    )
    .expect("writing consumer Cixfile");
    for name in ["producer", "consumer"] {
        fs::write(doc.base.join(name).join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
            .expect("writing compose member lock");
    }
    let members = [
        "producer/Cixfile",
        "producer/producer.py",
        "consumer/Cixfile",
        "consumer/consumer.py",
    ]
    .map(|path| doc.show_file(path))
    .join("");
    assert!(members.contains("RUNDIR /run/producer"));
    assert_eq!(members.matches("STATEDIR /var/lib/shared").count(), 2);
    assert!(members.contains("START consumer-probe /run/upstream/service.sock"));
    let producer_build = doc.sh(
        "producer_v1=$(cix build producer | jq -r '.[\"producer-v1\"]'); cix tag \"$producer_v1\" producer:current; printf '%s\\n' \"$producer_v1\"",
        true,
    );
    let first_producer = producer_build
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("producer v1 build printed its captured item")
        .to_owned();
    assert!(first_producer.ends_with("-cix-item-producer-v1"));
    let consumer_build = doc.sh(
        "consumer_item=$(cix build consumer -t v1 | jq -r '.consumer'); printf '%s\\n' \"$consumer_item\"",
        true,
    );
    let consumer = consumer_build
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("consumer build printed its captured item")
        .to_owned();
    assert!(Path::new(&first_producer)
        .join("bin/producer-probe")
        .is_file());
    assert!(Path::new(&consumer).join("bin/consumer-probe").is_file());

    doc.para("Before asking compose to project anything, run the exact two item programs against a tour-local Unix socket. A Unix socket is a filesystem entry used for local process-to-process messages; unlike a TCP port, it has a pathname. The consumer waits for that pathname, sends `ping`, and asserts the producer's versioned reply.");
    let unix_receipt = doc.sh_with_env(
        "set -eu; edge_dir=$(mktemp -d /tmp/cix-tour-edge-XXXXXX); edge_socket=$edge_dir/service.sock; \"$producer_v1/bin/producer-probe\" \"$edge_socket\" v1 >producer.log 2>&1 & producer_pid=$!; trap 'kill -TERM \"$producer_pid\" 2>/dev/null || true' EXIT; ready=; for attempt in $(seq 1 100); do if test -S \"$edge_socket\"; then ready=yes; break; fi; sleep 0.05; done; if test \"$ready\" != yes; then cat producer.log; exit 1; fi; \"$consumer_item/bin/consumer-probe\" \"$edge_socket\"; test -S \"$edge_socket\" && printf '%s is a Unix socket\\n' \"$edge_socket\"; kill -TERM \"$producer_pid\"; wait \"$producer_pid\" 2>/dev/null || true; trap - EXIT; cat producer.log; rm -rf \"$edge_dir\" producer.log",
        &[("producer_v1", &first_producer), ("consumer_item", &consumer)],
        true,
    );
    assert!(unix_receipt.contains("producer v1 received ping"));
    assert!(unix_receipt.contains("service.sock is a Unix socket"));

    fs::write(
        doc.base.join("compose.json"),
        r#"{
  "cixCompose": 1,
  "name": "tour-stack",
  "logNamespace": true,
  "children": {
    "producer": {
      "item": "producer:current",
      "update": "track",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    },
    "consumer": {
      "item": "consumer:v1",
      "update": "pin",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    }
  },
  "edges": {
    "producer-api": {
      "producer": {"child": "producer", "path": "/run/producer"},
      "consumers": {"consumer": {"path": "/run/upstream"}}
    }
  }
}
"#,
    )
    .expect("writing compose fixture");
    doc.para("The compose file supplies host policy without rebuilding either item. The `producer-api` edge gives the producer a writable `/run/producer` directory and bind-projects that same directory at `/run/upstream` in the consumer; therefore the producer creates `/run/producer/service.sock` and the consumer opens `/run/upstream/service.sock`. The edge unit owns a private group shared by those two dynamic users and starts the producer unit before the consumer unit. That ordering is not an application-readiness gate, so `consumer.py` still retries until the socket accepts connections.");
    let compose = doc.show_file("compose.json");
    assert!(compose.contains("\"shared\": \"payload\""));
    assert!(compose.contains("\"producer-api\""));
    assert!(compose.contains("\"path\": \"/run/upstream\""));
    assert!(compose.contains("\"update\": \"track\""));
    assert!(compose.contains("\"update\": \"pin\""));
    assert!(compose.contains("\"logNamespace\": true"));
    let checked = doc.sh("cix compose check compose.json", true);
    assert_eq!(
        checked.trim(),
        "compose tour-stack: 2 services, 1 edges, valid"
    );

    doc.para("`update: \"track\"` re-resolves the producer tag on every check, diff, or activation. `update: \"pin\"` reuses the consumer's existing `cix.lock` entry until `cix up --update-lock consumer compose.json` explicitly refreshes it; pin is also the default. Here `payload` is the compose-local volume identity, scoped below the `tour-stack` root. Every participating item must declare the mapped path as writable state (both do), and compose rejects incompatible roles. Its host backing is `/var/lib/cix-compose/tour-stack/shared/payload`, mode 2770 with setgid and a private supplemental group containing both services. Rollback and ordinary `down` retain the data; `sudo cix down tour-stack --purge --yes` removes it.");

    write_resolved_compose_lock_entries(
        &doc,
        &doc.base.join("compose.json"),
        &[
            ("producer", "producer:current"),
            ("consumer", "consumer:v1"),
        ],
    );
    let lock = doc.show_file("cix.lock");
    assert!(lock.contains(&first_producer));
    assert!(lock.contains(&consumer));
    doc.para("Only `cix up compose.json` writes this resolved `cix.lock`; commit it with the compose file. `cix compose check` and `cix compose diff` are read-only with respect to the lock. Because this harness cannot perform root activation, it materialized the same checked v1 resolution before displaying the lock, then runs the real dry-build below. No root profile is active for this tour stack, so the first diff compares against an empty baseline and `-` means that no prior service item exists.");
    let initial = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(
        initial.contains("cix-tour\x2dstack-producer.service"),
        "{initial}"
    );
    assert!(
        initial.contains("cix-tour\x2dstack-consumer.service"),
        "{initial}"
    );
    let initial_producer_line = initial
        .lines()
        .find(|line| line.starts_with("service producer:"))
        .expect("initial diff prints producer path");
    assert!(initial_producer_line.ends_with("-cix-item-producer-v1"));

    doc.para("`cix run` is the one-service form of the same compiler: its installable becomes a single compose child `item`; `-e NAME=value`, `-p name=value`, and `--dir path=materialization` correspond to that child's `env`, `bind`, and `dirs` maps. `--schedule` maps to `schedule`, and `--closed-root` selects the same generation option. Compose additionally supplies stable child names, edges, shared data, secrets, and retained generations.");
    let unary = doc.sh(
        "unit=$(cix run producer:current --user --detach); printf '%s\\n' \"$unit\"",
        true,
    );
    let unary_unit = unary
        .lines()
        .find(|line| line.starts_with("cix-run-producer-") && line.ends_with(".service"))
        .expect("unary run printed a producer unit")
        .to_owned();
    doc.sh_with_env(
        &idempotent_user_stop_command("$unit"),
        &[("unit", &unary_unit)],
        true,
    );
    wait_for_user_units_gone([unary_unit.as_str()]).expect("unary compose receipt unloads");
    stop_empty_cix_run_slice("the unary compose receipt");

    doc.para("Now change only the tracked producer, build a visibly named v2 item, and move the stable `producer:current` tag to it. The pinned consumer remains at the v1 lock entry. The second dry diff resolves the tracked tag and builds a candidate generation without touching the active system manager, profile, or lock.");
    doc.sh(
        "sed -i 's/producer-v1/producer-v2/; s/service.sock v1/service.sock v2/' producer/Cixfile",
        true,
    );
    let producer_v2 = doc.sh(
        "producer_v2=$(cix build producer | jq -r '.[\"producer-v2\"]'); cix tag \"$producer_v2\" producer:current; printf '%s\\n' \"$producer_v2\"",
        true,
    );
    let second_producer = producer_v2
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("producer v2 build printed its captured item")
        .to_owned();
    assert_ne!(first_producer, second_producer);
    assert!(second_producer.ends_with("-cix-item-producer-v2"));
    let changed = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(changed.contains(&second_producer), "{changed}");
    let changed_producer_line = changed
        .lines()
        .find(|line| line.starts_with("service producer:"))
        .expect("changed diff prints producer path");
    assert!(changed_producer_line.ends_with("-cix-item-producer-v2"));
    assert_ne!(initial_producer_line, changed_producer_line);

    doc.para("`diff` builds a candidate generation directory in the Nix store so it can compare complete unit files, but it neither adds that directory to the root profile nor guarantees retention across garbage collection. An activation creates a generation of `/nix/var/nix/profiles/cix-compose-tour-stack`; after activation, list every retained generation with `sudo nix-env -p /nix/var/nix/profiles/cix-compose-tour-stack --list-generations`. The profile and per-generation GC roots retain the generation and every referenced item closure.");

    doc.para("## Activation and rollback require root");
    doc.para("The supported activation command is `sudo env CIX_STATE_DIR=/var/lib/cix-index cix up compose.json`. It resolves according to update policy, writes `cix.lock`, builds and roots the generation, links its units below `/etc/systemd/system`, reloads the system manager, and starts `cix-tour-stack.target`; success prints `activated tour-stack from /nix/store/<hash>-cix-compose-tour-stack`. This harness stops at check and diff because those host-wide changes require root. The [stack VM scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/lib.nix) executes that exact up → selective change → diff → rollback → down lifecycle, and [the dirs scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix) asserts both writers see the shared setgid directory.");
    doc.para("`sudo cix rollback tour-stack` moves the profile to its preceding generation and activates those earlier unit definitions and resolved item references. It does not rewind shared or private data, secret-file contents, the mutable `producer:current` tag, or `cix.lock`. Activation is ordered but not transactional: if systemd fails partway, cix reports the failure and the selected profile remains available for an explicit rollback.");

    fs::write(
        doc.base.join("pod-fragment.json"),
        r#"{
  "children": {
    "workers": {
      "network": "pod",
      "children": {
        "producer": {"item": "producer:current"},
        "consumer": {"item": "consumer:v1"}
      }
    }
  }
}
"#,
    )
    .expect("writing pod compose fragment");
    fs::write(
        doc.base.join("logging-fragment.json"),
        "{\n  \"logNamespace\": true\n}\n",
    )
    .expect("writing logging compose fragment");
    doc.para("## Optional pod and journal grouping");
    doc.para("A group is an inline child with its own `children`. Putting `network: \"pod\"` on that group places exactly its descendant services in one private network namespace; the setting does not change the Unix edge above.");
    let pod = doc.show_file("pod-fragment.json");
    assert!(pod.contains("\"network\": \"pod\""));
    doc.para("At the compose root, `logNamespace: true` gives the tree a systemd journal namespace named `cix-tour-stack`. It isolates the stack's journal storage; cix still selects entries using the stamped `CIX_COMPOSITE` and `CIX_SERVICE` fields.");
    doc.show_file("logging-fragment.json");
    let log_selector = doc.sh("cix logs tour-stack/consumer --explain", true);
    assert!(log_selector.contains("journalctl CIX_COMPOSITE=tour-stack CIX_SERVICE=consumer"));
    doc.para("`cix logs tour-stack/consumer -n 20` reads the entries for only that child; `cix logs tour-stack -n 20` omits the service field and reads the whole tree. The consumer's stdout line demonstrated above is the application record those selectors retrieve after system activation. The [network-namespace scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/netns.nix) and [observability scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/observability.nix) carry the privileged pod and namespaced-journal receipts.");
    doc.finish()
}
