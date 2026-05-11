import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';
import { resolve } from 'path';

export default defineConfig({
    plugins: [
        dts({
            include: ['guest-js/**/*.ts'],
            outDir: 'dist-js',
            entryRoot: 'guest-js',
        }),
    ],
    build: {
        lib: {
            entry: resolve(__dirname, 'guest-js/index.ts'),
            formats: ['es', 'cjs'],
            fileName: (format) => (format === 'es' ? 'index.js' : 'index.cjs'),
        },
        outDir: 'dist-js',
        emptyOutDir: true,
        sourcemap: true,
        rollupOptions: {
            external: ['@tauri-apps/api', /^@tauri-apps\/api\//],
        },
    },
});
