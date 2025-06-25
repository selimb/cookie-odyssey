import path from "path";

const TEMPLATES_DIR = path.join(process.cwd(), "templates");

await Bun.build({
  // entrypoints: ["templates/common/js_utils.ts", "templates/common/datetime.ts"],
  // outdir: "static/js",
  entrypoints: ["assets/js/app.ts"],
  outdir: "assets/dist",
  splitting: false,
  format: "esm",
  target: "browser",
  // XXX naming
  // XXX minify
});
