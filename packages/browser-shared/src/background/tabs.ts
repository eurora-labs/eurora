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

export async function getCurrentTabIcon(): Promise<string> {
	try {
		// Get the active tab in the current window
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

		if (!activeTab || !activeTab.favIconUrl) {
			return '';
		}

		// If it's a data URL (already base64), return it directly
		if (activeTab.favIconUrl.startsWith('data:')) {
			// Extract base64 part from data URL
			const base64Match = activeTab.favIconUrl.match(/^data:image\/[^;]+;base64,(.+)$/);
			return base64Match ? base64Match[1] : '';
		}

		// If it's a chrome:// or chrome-extension:// URL, we can't fetch it
		if (
			activeTab.favIconUrl.startsWith('chrome://') ||
			activeTab.favIconUrl.startsWith('chrome-extension://')
		) {
			return '';
		}

		// Fetch the favicon and convert to base64
		const response = await fetch(activeTab.favIconUrl);
		if (!response.ok) {
			throw new Error(`Failed to fetch favicon: ${response.status}`);
		}

		const blob = await response.blob();
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.onloadend = () => {
				const result = reader.result as string;
				// Extract base64 part from data URL
				resolve(result);
			};
			reader.onerror = reject;
			reader.readAsDataURL(blob);
		});
	} catch (error) {
		console.error('Error getting current tab icon:', error);
		return '';
	}
}
