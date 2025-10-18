/**
 * Builds both JS and CSS, but it's easier to just call it "JS".
 *
 * We don't bother with minifying because our assets are relatively small, and not minifying
 * them makes debugging in the browser way easier (without bothering with source maps).
 */
import path from "node:path";

import { $, type BuildOutput } from "bun";

const OUT_DIR = "assets/dist";
const NAMING = "[dir]/[name].[hash].[ext]";

const manifest: Record<string, string> = {};

function accManifest(buildOutput: BuildOutput): void {
  for (const output of buildOutput.outputs) {
    const relativePath = path.relative(OUT_DIR, output.path);
    // Matches `NAMING` above.
    const manifestKey = relativePath.replace(`.${output.hash}`, "");
    manifest[manifestKey] = relativePath;
  }
}

let result = await Bun.build({
  entrypoints: ["assets/js/app.ts"],
  outdir: "assets/dist/js",
  splitting: false,
  format: "esm",
  target: "browser",
  naming: NAMING,
});
accManifest(result);

result = await Bun.build({
  entrypoints: ["assets/css/vendor/lightgallery.css"],
  outdir: "assets/dist/vendor",
  target: "browser",
  naming: NAMING,
});
accManifest(result);

await $`tailwindcss -i assets/css/app.css -o tmp/app.css`.quiet();

result = await Bun.build({
  entrypoints: ["tmp/app.css"],
  outdir: "assets/dist/css",
  target: "browser",
  naming: NAMING,
});
accManifest(result);

await Bun.write(
  path.join(OUT_DIR, "manifest.json"),
  JSON.stringify(manifest, null, 2),
);

export {};
