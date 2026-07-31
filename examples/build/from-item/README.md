# Copy from a tagged cix item

This is D65's cross-image-copy bridge. First tag the producer, then build this consumer:

```sh
cix build ../../pack/nginx -t v1
cix build .
```

`FROM my-nginx:v1 AS nginx` resolves the local index tag and writes its immutable store path
and NAR hash to `Cixfile.lock`. The source binder is a tree, so the consumer can copy the
producer's `/etc/nginx/nginx.conf` without inheriting its root filesystem. Rebuilding after the
tag moves keeps the lock pin; use `cix build --update-lock nginx` to move deliberately.
