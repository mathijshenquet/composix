Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The Docker-faithful release and the nixpkgs twin declare all four sockets, including `udp:443`; Alpine layers, fixed identity, and mode-setting dissolve into Nix and systemd. → case
- The 2026-08-05 independent sandbox FETCH returned the same 769-byte body for two different raw GitHub assets and therefore tripped their stable EXPECT pins; ordinary host curl retrieved the expected distinct assets. Reproduce from a clean builder before changing any pin. → evidence
