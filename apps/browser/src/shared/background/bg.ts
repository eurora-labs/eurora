import { matchSite } from './match';
import { loadRegistry } from './registry';
import browser from 'webextension-polyfill';

const INJECTABLE_SCHEME = /^(https?|file):/;

export function isInjectableUrl(url: string | undefined): url is string {
	return !!url && INJECTABLE_SCHEME.test(url);
}

export async function webNavigationListener(tabId: number, url: string, frameId: number) {
	if (frameId !== 0) return;
	if (!isInjectableUrl(url)) return;
	await injectIntoTab(tabId, url);
}

export async function injectIntoAllTabs() {
	const tabs = await browser.tabs.query({});
	await Promise.all(
		tabs.map(async (tab) => {
			if (tab.id === undefined || tab.discarded) return;
			if (!isInjectableUrl(tab.url)) return;
			await injectIntoTab(tab.id, tab.url);
		}),
	);
}

async function injectIntoTab(tabId: number, url: string) {
	try {
		const [check] = await browser.scripting.executeScript({
			target: { tabId, frameIds: [0] },
			func: () => document.documentElement.hasAttribute('eurora-ext-ready'),
		});
		if (check?.result === true) return;

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
		const message = site
			? {
					type: 'SITE_LOAD',
					siteId: site.id,
					chunk: `scripts/content/${site.chunk}`,
					defaultChunk,
					commonChunk,
				}
			: {
					type: 'SITE_LOAD',
					siteId: 'default',
					chunk: defaultChunk,
					defaultChunk,
					commonChunk,
				};

		await browser.tabs.sendMessage(tabId, message).catch((error) => {
			console.error('Failed to send SITE_LOAD message:', error);
		});
	} catch (error) {
		console.error('BG injection error: ', error);
	}
}
