import { getCurrentTab } from './tabs';
import browser from 'webextension-polyfill';

export async function sendMessageWithRetry(
	tabId: number,
	message: any,
	maxRetries: number = 5,
	delayMs: number = 500,
): Promise<any> {
	let lastError: any;
	for (let attempt = 0; attempt < maxRetries; attempt++) {
		try {
			return await browser.tabs.sendMessage(tabId, message);
		} catch (error: any) {
			lastError = error;
			const isLastAttempt = attempt === maxRetries - 1;
			const isConnectionError =
				error?.message?.includes('Receiving end does not exist') ||
				browser.runtime.lastError?.message?.includes('Receiving end does not exist');

			if (isConnectionError && !isLastAttempt) {
				await new Promise((resolve) => setTimeout(resolve, delayMs));
				continue;
			}
			throw error;
		}
	}
	throw lastError ?? new Error('sendMessageWithRetry: no attempts made');
}

export async function handleMessage(messageType: string) {
	try {
		const activeTab = await getCurrentTab();

		if (!activeTab || !activeTab.id) {
			return { success: false, data: 'No active tab found', kind: 'Error' };
		}

		const response = await sendMessageWithRetry(activeTab.id, {
			type: messageType,
		});

		return { success: true, ...response };
	} catch (error) {
		console.error('Error handling native message of type: ', messageType, error);
		return {
			kind: 'Error',
			success: false,
			data: String(error),
		};
	}
}
