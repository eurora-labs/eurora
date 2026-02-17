import browser from 'webextension-polyfill';
export type Entry = { id: string; chunk: string; patterns: string[] };

let REGISTRY: Entry[] | null = null;

export async function loadRegistry(): Promise<Entry[]> {
	if (REGISTRY) return REGISTRY;
	const url = browser.runtime.getURL('scripts/content/registry.json');
	const res = await fetch(url);
	if (!res.ok) {
		throw new Error(`Failed to load registry: ${res.status} ${res.statusText}`);
	}
	const data = await res.json();
	if (!Array.isArray(data)) {
		throw new Error('Registry data is not an array');
	}

	REGISTRY = data as Entry[];
	return REGISTRY!;
}
