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
