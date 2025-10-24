import browser from 'webextension-polyfill';
import { getCurrentTab } from './tabs.js';

/**
 * Sends a message to a tab with retry logic to handle content script initialization delays
 */
export async function sendMessageWithRetry(
	tabId: number,
	message: any,
	maxRetries: number = 5,
	delayMs: number = 500,
): Promise<any> {
	for (let attempt = 0; attempt < maxRetries; attempt++) {
		try {
			const response = await browser.tabs.sendMessage(tabId, message);
			return response;
		} catch (error: any) {
			const isLastAttempt = attempt === maxRetries - 1;
			const isConnectionError =
				error?.message?.includes('Receiving end does not exist') ||
				browser.runtime.lastError?.message?.includes('Receiving end does not exist');

			if (isConnectionError && !isLastAttempt) {
				console.log(`Content script not ready, retrying (${attempt + 1}/${maxRetries})...`);
				await new Promise((resolve) => setTimeout(resolve, delayMs));
				continue;
			}
			throw error;
		}
	}
}

export async function handleGenerateAssets() {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.id) {
			return { success: false, data: 'No active tab found', kind: 'Error' };
		}

		const response = await sendMessageWithRetry(activeTab.id, {
			type: 'GENERATE_ASSETS',
		});

		return { success: true, ...response };
	} catch (error) {
		console.error('Error generating report:', error);
		return {
			kind: 'Error',
			success: false,
			data: String(error),
		};
	}
}

export async function handleGenerateSnapshot() {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.id) {
			return { success: false, error: 'No active tab found' };
		}

		const response = await sendMessageWithRetry(activeTab.id, {
			type: 'GENERATE_SNAPSHOT',
		});

		return { success: true, ...response };
	} catch (error) {
		console.error('Error generating snapshot:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}
