Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: current

- The Dockerfile's copied `/entrypoint.sh` is absent from the supplied context, so its arbitrary-argument behavior cannot be audited. The service starts `traefik --ping=true` directly and the receipt proves only that ping endpoint, not a reverse-proxy configuration. → evidence + case
- The faithful builder uses GitHub's release-API asset endpoint because the Dockerfile's browser-download URL returned 404 from the FETCH sandbox; the selected version, architecture name, and published digest remain the same. → case
- Both FETCH steps intentionally retain the identical copy-pasted `EXPECT`. Warm builds never validated it, proving the [EXPECT-versus-recorded-pin defect](../../../../cips/open-questions.md#expect-not-validated-against-the-recorded-pin-on-warm-builds); correcting either value before the product fix would destroy this case's reproduction. → language (EXPECT validation)
- The first FETCH pins mutable GitHub release-metadata JSON whose download counters change, making cold refetches EXPECT-hostile. Normalize it to the consumed fields, or bypass it for the stable asset URL, in the same product round that fixes EXPECT validation. → case
