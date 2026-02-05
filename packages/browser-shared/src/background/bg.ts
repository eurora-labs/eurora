import { matchSite } from './match.js';
import { loadRegistry } from './registry.js';
import browser from 'webextension-polyfill';

export async function webNavigationListener(tabId: number, url: string, frameId: number) {
	try {
		if (frameId !== 0 || !url) return;
		const u = new URL(url);
		const entries = await loadRegistry();
		const site = matchSite(u.hostname, entries);

		await browser.scripting.executeScript({
			target: { tabId, frameIds: [0] },
			world: 'ISOLATED',
			files: ['scripts/content/bootstrap.js'],
			injectImmediately: true,
		});

		const defaultChunk = 'scripts/content/sites/_default/index.js';
		const commonChunk = 'scripts/content/sites/_common/index.js';
		if (!site) {
			await browser.tabs
				.sendMessage(tabId, {
					type: 'SITE_LOAD',
					siteId: 'default',
					chunk: defaultChunk,
					defaultChunk,
					commonChunk,
				})
				.catch((error) => {
					console.error('Failed to send SITE_LOAD message:', error);
				});
			return;
		}

		// Optional: request origin permission only for known sites that need fetch
		// await chrome.permissions.request({ origins: [u.origin + '/*'] }).catch(() => {});

		await browser.tabs
			.sendMessage(tabId, {
				type: 'SITE_LOAD',
				siteId: site.id,
				// chunk paths are already content-side relative inside dist
				chunk: `scripts/content/${site.chunk}`,
				defaultChunk,
				commonChunk,
			})
			.catch((error) => {
				console.error('Failed to send SITE_LOAD message:', error);
			});
	} catch (error) {
		if (url?.startsWith('http')) {
			console.error('BG injection error: ', error);
		}
	}
}
