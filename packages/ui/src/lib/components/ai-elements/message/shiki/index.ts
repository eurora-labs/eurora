export {
	getShikiWorkerClient,
	type HighlightResult,
	type ShikiWorkerClient,
} from './shiki-worker-client.svelte.js';
export { isLanguageSupported } from './languages.js';

import { getShikiWorkerClient } from './shiki-worker-client.svelte.js';

/**
 * Default warmup language set: the most common languages we see in chat
 * code blocks. Loading these eagerly at app start avoids paying the
 * grammar-load round-trip on the user's first code block.
 */
export const DEFAULT_WARMUP_LANGS: readonly string[] = [
	'typescript',
	'javascript',
	'rust',
	'python',
	'json',
];

/**
 * Boot the Shiki worker and pre-load the listed languages. Safe to call
 * outside a worker-capable environment — it no-ops there.
 */
export function warmupShikiHighlighter(
	langs: readonly string[] = DEFAULT_WARMUP_LANGS,
): void {
	if (typeof Worker === 'undefined') return;
	try {
		getShikiWorkerClient().warmup([...langs]);
	} catch (err) {
		console.warn('[shiki] warmup skipped:', err);
	}
}
