await Bun.build({
  entrypoints: ["assets/js/app.ts"],
  outdir: "assets/dist/js",
  splitting: false,
  format: "esm",
  target: "browser",
  // XXX naming
  // XXX minify
});

await Bun.build({
  entrypoints: ["assets/css/vendor/lightgallery.css"],
  outdir: "assets/dist/vendor",
  target: "browser",
  // XXX naming
  // XXX minify
});

export {};
