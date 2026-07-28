import { cp, mkdir } from "node:fs/promises";

await mkdir("dist", { recursive: true });
await cp("index.html", "dist/index.html");
await cp("src/main.js", "dist/main.js");
