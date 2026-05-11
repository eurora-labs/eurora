import { defineConfig } from 'vite';
import { resolve } from 'path';
import { rename, rm } from 'fs/promises';

// Minified IIFE bundle published alongside the npm tarball at the package
// root. Externalizes `@tauri-apps/api` against the global the Tauri runtime
// injects into the webview (`window.__TAURI__`).
//
// Vite refuses to write its outDir into the project root (it would risk
// overwriting source files), so we emit into a throwaway `dist-iife/` and
// hoist the single output up to `api-iife.js` after the bundle closes.
export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'guest-js/index.ts'),
            formats: ['iife'],
            name: '__TAURI_PLUGIN_APPAUTH__',
            fileName: () => 'api-iife.js',
        },
        outDir: 'dist-iife',
        emptyOutDir: true,
        minify: true,
        sourcemap: false,
        rollupOptions: {
            external: ['@tauri-apps/api', /^@tauri-apps\/api\//],
            output: {
                globals: (id) =>
                    id === '@tauri-apps/api' || id.startsWith('@tauri-apps/api/')
                        ? '__TAURI__'
                        : id,
            },
        },
    },
    plugins: [
        {
            name: 'hoist-iife-to-root',
            closeBundle: async () => {
                const stagingDir = resolve(__dirname, 'dist-iife');
                await rename(resolve(stagingDir, 'api-iife.js'), resolve(__dirname, 'api-iife.js'));
                await rm(stagingDir, { recursive: true, force: true });
            },
        },
    ],
});
