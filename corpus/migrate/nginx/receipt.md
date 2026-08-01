# nginx migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`LOGSDIR`, explicit artifact `bin/`, and quote-aware `START`).

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## `./check.sh cix`

```text
cix item /nix/store/xlnmhf6gwsl8c41q7f8iq241vgs5r102-cix-item-nginx
```

Exit status: 0. The HTTP probe passed; `-g 'daemon off;'` is now one argv word.
