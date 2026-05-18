// Resolve filesystem paths relative to the repo root.
//
// `scripts/lib/paths.mjs` lives two directories below the workspace root,
// so we walk up from this module's URL. Doing it once here keeps every
// caller free of `import.meta.url` boilerplate and makes the assumption
// explicit in one place.

import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

export const repoRoot = resolve(here, '..', '..');

export function fromRepoRoot(...segments) {
	return resolve(repoRoot, ...segments);
}
