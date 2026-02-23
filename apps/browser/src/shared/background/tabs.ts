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

async function fetchFaviconAsBase64(faviconUrl: string): Promise<string> {
	const response = await fetch(faviconUrl, { credentials: 'include' });
	if (!response.ok) {
		throw new Error(`Failed to fetch favicon: ${response.status}`);
	}

	const blob = await response.blob();
	return await new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onloadend = () => {
			const dataUrl = reader.result as string;
			const base64 = dataUrl.split(',')[1] || '';
			resolve(base64);
		};
		reader.onerror = reject;
		reader.readAsDataURL(blob);
	});
}

// Safari often has null favIconUrl — fall back to content script DOM query
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

		if (!faviconUrl && activeTab.id !== undefined) {
			faviconUrl = (await getFaviconUrlFromContentScript(activeTab.id)) ?? undefined;
		}

		if (!faviconUrl) {
			return '';
		}

		if (faviconUrl.startsWith('data:')) {
			const base64Match = faviconUrl.match(/^data:image\/[^;]+;base64,(.+)$/);
			return base64Match ? base64Match[1] : '';
		}

		// chrome:// and chrome-extension:// URLs can't be fetched
		if (faviconUrl.startsWith('chrome://') || faviconUrl.startsWith('chrome-extension://')) {
			return '';
		}

		return await fetchFaviconAsBase64(faviconUrl);
	} catch (error) {
		console.error('Error getting current tab icon:', error);
		return '';
	}
}
