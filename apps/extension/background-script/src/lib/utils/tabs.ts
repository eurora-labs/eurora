/**
 * Browser tab manipulation utilities
 */

/**
 * Gets the current active tab in the current window
 * @returns {Promise<chrome.tabs.Tab>} The current active tab
 */
export async function getCurrentTab() {
	try {
		const [tab] = await chrome.tabs.query({
			active: true,
			currentWindow: true,
		});

		return tab;
	} catch (error) {
		console.error('Error getting current tab:', error);
		return null;
	}
}

/**
 * Gets all tabs matching a URL pattern
 * @param {string} urlPattern - URL pattern to match
 * @returns {Promise<chrome.tabs.Tab[]>} Array of matching tabs
 */
export async function getTabsByUrlPattern(urlPattern) {
	try {
		return await chrome.tabs.query({
			url: urlPattern,
		});
	} catch (error) {
		console.error('Error getting tabs by URL pattern:', error);
		return [];
	}
}

/**
 * Focus on a specific tab by ID
 * @param {number} tabId - The ID of the tab to focus
 * @returns {Promise<boolean>} True if successful
 */
export async function focusTab(tabId) {
	try {
		await chrome.tabs.update(tabId, { active: true });
		await chrome.windows.update((await chrome.tabs.get(tabId)).windowId, { focused: true });
		return true;
	} catch (error) {
		console.error('Error focusing tab:', error);
		return false;
	}
}
