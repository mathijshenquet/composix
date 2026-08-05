use super::super::*;

pub(crate) fn chapter_naming_distribution() -> String {
    let mut doc = Doc::new("naming-distribution");
    let publisher = doc.state_dir.clone();
    let consumer = doc.base.join("consumer-state");

    doc.para("You will name immutable items, move and remove those names, then serve one local index and pull from it into another. A **family** is the slash-grouped prefix of related tag names, such as `guide/` in `guide/web:v1`. It is a naming convention except where `cix build --namespace` creates member names and `cix ls guide/` filters them; it does not create an access-control, storage, or distribution boundary.");

    doc.para("## One demystifying aside: an item is a tree");
    doc.para("Normally `cix build` writes this tree for you. At the boundary, however, an item is simply a Nix store tree with `cix-manifest.json`. This hand-written manifest intentionally makes a taggable inspection fixture, not a runnable service: `message` is data rather than an executable. `nix store add` recursively serializes the directory as a Nix archive, copies it to a content-addressed store path, and prints that path; it neither validates the cix manifest nor protects the result from garbage collection.");
    let first = fixture(&mut doc, "my-app-v1", "hello from my app v1");

    doc.para("## Names come after builds");
    doc.para("The store path already has its complete content identity. A tag is a mutable pointer added afterwards, and its source-then-destination syntax is `cix tag <item-or-existing-ref> <new-bare-ref>`. Each local tag is also a **GC root**, a durable reference that keeps the item from Nix garbage collection; cleanup can reclaim the item only after every cix tag and other root is gone. The explicit `:tag` suffix is mandatory—there is no implicit `latest`.");
    doc.sh("cix tag my-app:v1 guide/web:v1", true);
    doc.sh("cix tag my-app:v1 guide/web:stable", true);
    let family = doc.sh("cix ls -l guide/", true);
    assert!(family.contains("guide/web:v1"));
    assert!(family.contains("guide/web:stable"));
    assert!(family.contains(&first));

    let inspected = doc.sh(
        "cix inspect guide/web:v1 | jq '{kind, reference, storePath, systems:(.outputs | keys)}'",
        true,
    );
    assert!(inspected.contains("\"kind\": \"artifact\""));
    assert!(inspected.contains("\"reference\": \"guide/web:v1\""));
    assert!(inspected.contains(&first));
    assert!(inspected.contains(std::env::consts::ARCH));
    doc.para("The inspection word `artifact` means the item-facing side of `cix inspect`, not a fourth Cixfile block kind alongside SERVICE, APP, and ITEM. `systems` comes from the current Nix store platform that `cix tag` records in the index output slot; the hand-written manifest did not declare it.");

    doc.para("Names move without rewriting item bytes. Create the destination pointer and then remove the source with `cix untag`; the old store path remains as long as any other root still reaches it.");
    doc.sh(
        "cix tag guide/web:v1 guide/web:release && cix untag guide/web:stable",
        true,
    );
    let moved_names = doc.sh("cix ls guide/", true);
    assert!(moved_names.contains("guide/web:release"));
    assert!(!moved_names.contains("guide/web:stable"));

    doc.para("Build each new immutable tree before pointing a tag at it. Version 2 differs by one payload line:");
    doc.sh(
        "mkdir my-app-v2 && printf '%s\\n' 'hello from my app v2' > my-app-v2/message && printf '%s\\n' '{\"cixManifest\":0,\"start\":[\"message\"]}' > my-app-v2/cix-manifest.json",
        true,
    );
    doc.show_file("my-app-v2/message");
    let second_added = doc.sh(
        "item_v2=$(nix store add my-app-v2); printf '%s\\n' \"$item_v2\"",
        true,
    );
    let second = second_added
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("nix store add printed the v2 item")
        .to_owned();
    doc.para("Moving `guide/web:v1` to a new build changes only that pointer. The immutable v1 path still exists wherever another root retains it.");
    doc.sh_with_env(
        "cix tag \"$item_v2\" guide/web:v1",
        &[("item_v2", &second)],
        true,
    );
    let moved = doc.sh("cix ls -l guide/", true);
    assert!(moved.contains(&second));
    assert!(moved.contains(&first));

    doc.para("## Serve and pull");
    doc.para("This demo runs publisher and consumer prompts as two logical shells with separate `CIX_STATE_DIR` indexes on one host; they share only the host's Nix daemon/store. On two machines, install Nix and cix on the consumer as in Chapter 1. `cix serve --with-store` exposes the publisher's bare tag database and additionally materializes a standard Nix binary cache containing the referenced closures.");
    doc.para("The qualified-reference grammar is `host:port/family/name:tag`: the host and optional port before the first slash are the origin, the middle slash components are the name, and the final colon introduces the mandatory tag. Name components use lower-case letters, digits, `.`, `_`, and `-`; there is no path escaping or default registry. For the command below, that becomes `127.0.0.1:8420/guide/web:v1`.");
    doc.para("The same ordinary URL is content-negotiated so humans and tools need no parallel API: a browser receives HTML, while cix sends the shown `Accept` header and receives the exact JSON index entry. The index maps the name to a store path; Nix then downloads its **closure**, the item plus every store path it references at runtime.");
    let listen = next_listen();
    doc.background(
        "publisher $",
        &format!("cix serve --with-store --listen {listen}"),
    );
    let server = start_server(&doc, &publisher, &listen);
    let entry = doc.sh_in(
        "publisher $",
        &publisher,
        &format!(
            "curl -s -H 'Accept: application/vnd.cix+json;version=1' http://{listen}/guide/web:v1 | jq '{{outputs, substituters}}'"
        ),
        true,
    );
    assert!(entry.contains("\"outputs\""));
    assert!(entry.contains(&second));
    assert!(entry.contains("/store"));

    doc.para("This localhost demo is deliberately unsigned: NAR hashes detect corruption, but they do not authenticate who published the bytes. Production adds TLS plus `cix serve --with-store --sign-key /etc/cix/cache.sec`; the corresponding public key must be trusted in the consumer's Nix `trusted-public-keys` configuration and may be advertised in the index entry. Do not infer production trust from the unsigned loopback receipt.");
    doc.para("`--as` adopts the qualified remote ref under a bare local name and stores its upstream origin in tag metadata. The pull copies the selected item closure from the advertised `/store` cache, verifies the recorded NAR hash and any configured signature policy, then creates the consumer's local GC-rooted tag. A later argument-free `cix pull` revisits every recorded upstream and downloads any closure whose tag moved.");
    let pulled = doc.sh_in(
        "consumer $",
        &consumer,
        &format!("cix pull {listen}/guide/web:v1 --as guide/web:v1"),
        true,
    );
    assert!(pulled.contains("updated 1 tag(s)"));
    let local = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(local.contains("guide/web:v1"));
    assert!(local.contains(&second));
    assert!(local.contains(&listen));

    doc.sh_in(
        "publisher $",
        &publisher,
        "mkdir my-app-v3 && printf '%s\\n' 'hello from my app v3' > my-app-v3/message && printf '%s\\n' '{\"cixManifest\":0,\"start\":[\"message\"]}' > my-app-v3/cix-manifest.json",
        true,
    );
    doc.show_file("my-app-v3/message");
    let third_added = doc.sh_in(
        "publisher $",
        &publisher,
        "item_v3=$(nix store add my-app-v3); printf '%s\\n' \"$item_v3\"",
        true,
    );
    let third = third_added
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("nix store add printed the v3 item")
        .to_owned();
    let output = doc.run_with_env(
        &publisher,
        "cix tag \"$item_v3\" guide/web:v1",
        &[("item_v3", &third)],
        true,
    );
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    doc.record("publisher $", "cix tag \"$item_v3\" guide/web:v1", &raw);
    let refreshed = doc.sh_in("consumer $", &consumer, "cix pull", true);
    assert!(refreshed.contains("updated 1 tag(s)"));
    let updated = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(updated.contains(&third));
    assert!(!updated.contains(&second));
    drop(server);

    doc.para("The positive model is deliberately small: each local cix index stores GC-rooted name-to-path records and optional upstream origins; qualified names select another served index; Nix substitution transfers the complete immutable closure.");
    doc.finish()
}
