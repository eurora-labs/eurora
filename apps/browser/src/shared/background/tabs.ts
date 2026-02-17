import browser from 'webextension-polyfill';

export async function getCurrentTab(): Promise<browser.Tabs.Tab | null> {
	try {
		const [tab] = await browser.tabs.query({
			active: true,
			currentWindow: true,
		});

		return tab ?? null;
	} catch (error) {
		console.error('Error getting current tab:', error);
		return null;
	}
}

export async function getTabsByUrlPattern(urlPattern: string): Promise<browser.Tabs.Tab[]> {
	try {
		return await browser.tabs.query({
			url: urlPattern,
		});
	} catch (error) {
		console.error('Error getting tabs by URL pattern:', error);
		return [];
	}
}

/**
 * Fetches a favicon URL and converts it to a base64 data URL.
 */
async function fetchFaviconAsBase64(faviconUrl: string): Promise<string> {
	const response = await fetch(faviconUrl, { credentials: 'include' });
	if (!response.ok) {
		throw new Error(`Failed to fetch favicon: ${response.status}`);
	}

	const blob = await response.blob();
	return await new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onloadend = () => {
			const result = reader.result as string;
			resolve(result);
		};
		reader.onerror = reject;
		reader.readAsDataURL(blob);
	});
}

/**
 * Gets the favicon URL by injecting a content script that queries the DOM.
 * This is needed for Safari where favIconUrl is often null.
 */
async function getFaviconUrlFromContentScript(tabId: number): Promise<string | null> {
	try {
		const results = await browser.scripting.executeScript({
			target: { tabId },
			func: () => {
				const selectors = [
					'link[rel="icon"]',
					'link[rel="shortcut icon"]',
					'link[rel="mask-icon"]',
					'link[rel="apple-touch-icon"]',
					'link[rel="apple-touch-icon-precomposed"]',
				];

				for (const sel of selectors) {
					const link = document.querySelector(sel) as HTMLLinkElement | null;
					if (link && link.href) {
						return link.href;
					}
				}

				// Fallback: /favicon.ico on same origin
				try {
					return new URL('/favicon.ico', window.location.origin).href;
				} catch {
					return null;
				}
			},
		});

		if (results && results[0] && results[0].result) {
			return results[0].result as string;
		}
		return null;
	} catch (error) {
		console.error('Error executing content script for favicon:', error);
		return null;
	}
}

export async function getCurrentTabIcon(activeTab: browser.Tabs.Tab): Promise<string> {
	try {
		if (!activeTab) {
			return '';
		}

		let faviconUrl = activeTab.favIconUrl;

		// If favIconUrl is not available (common on Safari), try getting it from the content script
		if (!faviconUrl && activeTab.id !== undefined) {
			faviconUrl = (await getFaviconUrlFromContentScript(activeTab.id)) ?? undefined;
		}

		if (!faviconUrl) {
			return '';
		}

		// If it's a data URL (already base64), return it directly
		if (faviconUrl.startsWith('data:')) {
			// Extract base64 part from data URL
			const base64Match = faviconUrl.match(/^data:image\/[^;]+;base64,(.+)$/);
			return base64Match ? base64Match[1] : '';
		}

		// If it's a chrome:// or chrome-extension:// URL, we can't fetch it
		if (faviconUrl.startsWith('chrome://') || faviconUrl.startsWith('chrome-extension://')) {
			return '';
		}

		// Fetch the favicon and convert to base64
		return await fetchFaviconAsBase64(faviconUrl);
	} catch (error) {
		console.error('Error getting current tab icon:', error);
		return '';
	}
}
