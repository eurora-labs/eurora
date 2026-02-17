import { resolve } from 'path';
import { readdirSync } from 'fs';
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';

const libDir = resolve(__dirname, 'src/lib');

function collectEntries(dir: string, base = ''): Record<string, string> {
	const entries: Record<string, string> = {};
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const rel = base ? `${base}/${entry.name}` : entry.name;
		if (entry.isDirectory()) {
			Object.assign(entries, collectEntries(resolve(dir, entry.name), rel));
		} else if (
			entry.name.endsWith('.ts') &&
			!entry.name.endsWith('.test.ts') &&
			!entry.name.endsWith('.spec.ts') &&
			!entry.name.endsWith('.d.ts')
		) {
			const key = rel.replace(/\.ts$/, '');
			entries[key] = resolve(dir, entry.name);
		}
	}
	return entries;
}

export default defineConfig({
	resolve: {
		alias: {
			$lib: resolve(__dirname, 'src/lib'),
		},
	},
	plugins: [
		dts({
			include: ['src/lib/**/*.ts'],
			exclude: ['src/**/*.test.ts', 'src/**/*.spec.ts'],
		}),
	],
	build: {
		lib: {
			entry: collectEntries(libDir),
			formats: ['es'],
		},
		outDir: 'dist',
		sourcemap: true,
		rollupOptions: {
			external: [/^@bufbuild\//, /^@connectrpc\//],
		},
	},
	test: {
		include: ['src/**/*.(test|spec).?(m)[jt]s?(x)'],
	},
});
