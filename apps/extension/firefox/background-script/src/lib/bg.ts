import { matchSite } from '@eurora/browser-shared/match';
import { loadRegistry } from '@eurora/browser-shared/registry';

browser.webNavigation.onCommitted.addListener(async ({ tabId, url, frameId }: any) => {
	if (frameId !== 0 || !url) return;
	const u = new URL(url);
	const entries = await loadRegistry();
	const site = matchSite(u.hostname, entries);
	if (!site) return;

	await browser.tabs.executeScript(tabId, {
		file: site.chunk,
		runAt: 'document_idle',
	});
});
