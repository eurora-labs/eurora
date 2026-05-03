#!/usr/bin/env node
// Renders manifest.template.xml -> manifest.dev.xml for local sideloading.
// The production installer (Phase 5) does its own rendering and never reads
// this output. Tokens default to dev-server values; override via env vars.

import { readFile, writeFile } from 'node:fs/promises';
import { relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const TEMPLATE = new URL('../manifest.template.xml', import.meta.url);
const OUTPUT = new URL('../manifest.dev.xml', import.meta.url);

const DEV_HOST = process.env.OFFICE_ADDIN_DEV_HOST ?? 'https://localhost:3000';
const DEFAULTS = {
	ADDIN_ID: process.env.OFFICE_ADDIN_ID ?? 'b0c1c6a4-4d0a-4c6f-9f3a-eee5eeee5eee',
	VERSION: process.env.OFFICE_ADDIN_VERSION ?? '1.0.0.0',
	SOURCE_LOCATION: process.env.OFFICE_ADDIN_SOURCE_LOCATION ?? `${DEV_HOST}/runtime.html`,
	ICON_BASE_URL: process.env.OFFICE_ADDIN_ICON_BASE_URL ?? `${DEV_HOST}/icons/`,
};

const template = await readFile(TEMPLATE, 'utf8');
const rendered = template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
	const value = DEFAULTS[key];
	if (value === undefined) {
		throw new Error(`Unknown manifest token: {{${key}}}`);
	}
	return value;
});

await writeFile(OUTPUT, rendered, 'utf8');
process.stdout.write(`rendered ${relative(process.cwd(), fileURLToPath(OUTPUT))}\n`);
