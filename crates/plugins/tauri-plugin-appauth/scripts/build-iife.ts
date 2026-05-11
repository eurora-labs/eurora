const result = await Bun.build({
    entrypoints: ["guest-js/index.ts"],
    format: "iife",
    naming: "api-iife.js",
    outdir: ".",
    minify: true,
    external: ["@tauri-apps/api"],
});

if (!result.success) {
    console.error("Build failed:");
    for (const log of result.logs) {
        console.error(log);
    }
    process.exit(1);
}
