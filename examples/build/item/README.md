# Pure ITEM tree

This is a D68 content-only artifact. It has no `cix-manifest.json`, so it cannot be run or
debugged. Build its selected member and inspect the assembled tree:

```sh
cix build .#welcome-assets
```

Tag it with `cix build . -t v1`, then use `FROM welcome-assets:v1 AS assets` in another
Cixfile and copy a declared path such as `${assets}/share/message.txt`.
