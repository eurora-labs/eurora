import { matchSite } from '@eurora/browser-shared/match';
import { loadRegistry } from '@eurora/browser-shared/registry';

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
