// scripts/zip.mjs
import pkg from '../package.json' assert { type: 'json' };
import archiver from 'archiver';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const distRoot = path.join(root, 'dist');
const releasesRoot = path.join(root, 'releases');

function ensureDir(p) {
	fs.mkdirSync(p, { recursive: true });
}

function listBrowsers() {
	const fromEnv = process.env.BROWSER;
	if (fromEnv) return [fromEnv];
	if (!fs.existsSync(distRoot)) return [];
	return fs
		.readdirSync(distRoot, { withFileTypes: true })
		.filter((d) => d.isDirectory())
		.map((d) => d.name);
}

async function zipDir({ inDir, outFile }) {
	await new Promise((resolve, reject) => {
		ensureDir(path.dirname(outFile));

		const output = fs.createWriteStream(outFile);
		const archive = archiver('zip', { zlib: { level: 9 } });

		output.on('close', resolve);
		output.on('error', reject);
		archive.on('error', reject);

		archive.pipe(output);

		archive.directory(inDir, false, (entry) => {
			const name = entry.name.replace(/\\/g, '/');
			if (name === '.DS_Store') return false;
			if (name.startsWith('__MACOSX/')) return false;
			if (name.endsWith('/.DS_Store')) return false;
			return entry;
		});

		archive.finalize();
	});
}

const version = pkg.version;
const browsers = listBrowsers();

ensureDir(releasesRoot);

for (const browser of browsers) {
	const inDir = path.join(distRoot, browser);
	if (!fs.existsSync(inDir)) continue;

	const outFile = path.join(releasesRoot, `eurora-extension-${browser}-v${version}.zip`);
	await zipDir({ inDir, outFile });
	process.stdout.write(`zipped: ${outFile}\n`);
}
