import browser from 'webextension-polyfill';
export type Entry = { id: string; chunk: string; patterns: string[] };

let REGISTRY: Entry[] | null = null;

export async function loadRegistry(): Promise<Entry[]> {
	if (REGISTRY) return REGISTRY;
	const url = browser.runtime.getURL('scripts/content/registry.json');
	const res = await fetch(url);
	REGISTRY = await res.json();
	return REGISTRY!;
}
