import browser from 'webextension-polyfill';

const INTERVAL_MS = 3000;
const TIMEOUT_MS = 2000;

let timer: ReturnType<typeof setInterval> | null = null;

export function startHeartbeat(): void {
	if (timer) return;
	timer = setInterval(ping, INTERVAL_MS);
}

export function stopHeartbeat(): void {
	if (timer) {
		clearInterval(timer);
		timer = null;
	}
}

async function ping(): Promise<void> {
	try {
		const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
		if (!tab?.id) return;

		const response = await Promise.race([
			browser.tabs.sendMessage(tab.id, { type: 'HEARTBEAT' }),
			new Promise<never>((_, reject) =>
				setTimeout(() => reject(new Error('heartbeat timeout')), TIMEOUT_MS),
			),
		]);

		if (response !== true) {
			console.warn('Heartbeat: unexpected response', response);
		}
	} catch {
		// Tab may not have content script injected — ignore
	}
}
