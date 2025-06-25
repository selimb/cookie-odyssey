await Bun.build({
  entrypoints: ["assets/js/app.ts"],
  outdir: "assets/dist",
  splitting: false,
  format: "esm",
  target: "browser",
  // XXX naming
  // XXX minify
});

export {};
