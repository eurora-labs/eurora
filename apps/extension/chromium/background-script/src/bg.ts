type Entry = { id: string; chunk: string; patterns: string[] };

let REGISTRY: Entry[] | null = null;

async function loadRegistry(): Promise<Entry[]> {
	if (REGISTRY) return REGISTRY;
	const url = chrome.runtime.getURL('scripts/content/registry.json');
	const res = await fetch(url);
	REGISTRY = await res.json();
	return REGISTRY!;
}

function matchSite(host: string, entries: Entry[]): Entry | null {
	// Precompute maps once per activation for O(1) exact / suffix
	const exact = new Map<string, Entry>();
	const suffix: [string, Entry][] = [];
	for (const e of entries) {
		for (const p of e.patterns) {
			if (p.startsWith('*.')) suffix.push([p.slice(2), e]);
			else exact.set(p, e);
		}
	}
	const hit = exact.get(host);
	if (hit) return hit;
	for (const [suf, e] of suffix) {
		if (host === suf || host.endsWith('.' + suf)) return e;
	}
	return null;
}

chrome.webNavigation.onCommitted.addListener(async ({ tabId, url, frameId }) => {
	try {
		if (frameId !== 0 || !url) return;
		const u = new URL(url);
		const entries = await loadRegistry();
		const site = matchSite(u.hostname, entries);

		await chrome.scripting.executeScript({
			target: { tabId, frameIds: [0] },
			world: 'ISOLATED',
			files: ['scripts/content/bootstrap.js'],
		});

		const defaultChunk = 'scripts/content/sites/_default/index.js';
		if (!site) {
			await chrome.tabs.sendMessage(tabId, {
				type: 'SITE_LOAD',
				siteId: 'default',
				chunk: defaultChunk,
				defaultChunk,
			});
			return;
		}

		// Optional: request origin permission only for known sites that need fetch
		// await chrome.permissions.request({ origins: [u.origin + '/*'] }).catch(() => {});

		await chrome.tabs.sendMessage(tabId, {
			type: 'SITE_LOAD',
			siteId: site.id,
			// chunk paths are already content-side relative inside dist
			chunk: `scripts/content/${site.chunk}`,
			defaultChunk,
		});
	} catch {
		/* no-op */
	}
});
