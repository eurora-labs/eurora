import {
	type IconCandidate,
	type IconLinkRecord,
	collectIconCandidatesFromLinks,
	isSupportedScheme,
	originFallbackCandidate,
	resolveBestCandidate,
	tabFaviconCandidate,
} from './favicon-ranker';
import browser from 'webextension-polyfill';

async function discoverDomCandidates(tabId: number): Promise<IconCandidate[]> {
	try {
		const results = await browser.scripting.executeScript({
			target: { tabId },
			func: () => {
				const links = document.querySelectorAll<HTMLLinkElement>('link[rel]');
				const out: Array<{
					href: string;
					rel: string;
					type: string;
					sizes: string;
				}> = [];
				links.forEach((link) => {
					if (!link.href) return;
					out.push({
						href: link.href,
						rel: link.rel || '',
						type: link.type || '',
						sizes: link.getAttribute('sizes') || '',
					});
				});
				return out;
			},
		});

		const raw = results?.[0]?.result;
		if (!Array.isArray(raw)) return [];
		return collectIconCandidatesFromLinks(raw as IconLinkRecord[]);
	} catch (error) {
		console.error('Favicon DOM discovery failed:', error);
		return [];
	}
}

/**
 * Resolves the best favicon for a tab as a base64 string. Returns "" if every
 * candidate fails. Does not depend on `tab.favIconUrl` being populated — that
 * field lags page navigation and is the source of the Wikipedia placeholder
 * regression. The DOM scrape and origin fallback together cover the common case.
 */
export async function resolveFaviconBase64(tab: browser.Tabs.Tab): Promise<string> {
	if (!tab) return '';
	if (tab.url && !isSupportedScheme(tab.url)) return '';

	const candidates: IconCandidate[] = [];

	if (tab.id !== undefined) {
		candidates.push(...(await discoverDomCandidates(tab.id)));
	}

	const tabCandidate = tabFaviconCandidate(tab.favIconUrl, candidates.length);
	if (tabCandidate) candidates.push(tabCandidate);

	const originCandidate = originFallbackCandidate(tab.url, candidates.length);
	if (originCandidate) candidates.push(originCandidate);

	return await resolveBestCandidate(candidates);
}
