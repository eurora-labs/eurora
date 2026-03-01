import { sendMessageWithRetry } from './messaging';
import { getCurrentTabIcon } from './tabs';
import { isSafari } from './util';
import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, NativeMetadata } from '../content/bindings';

declare const __DEV__: boolean;
const host = __DEV__ ? 'com.eurora.dev' : 'com.eurora.app';

let pollInterval: ReturnType<typeof setInterval> | null = null;
let polling = false;

export function startSafariPoller(): void {
	if (!isSafari()) return;
	if (pollInterval) return;
	pollInterval = setInterval(pollForRequests, 500);
}

export function stopSafariPoller(): void {
	if (pollInterval) {
		clearInterval(pollInterval);
		pollInterval = null;
	}
	polling = false;
}

async function pollForRequests(): Promise<void> {
	if (polling) return;
	polling = true;
	try {
		const response = (await browser.runtime.sendNativeMessage(host, {
			kind: {
				Request: {
					id: 0,
					action: 'POLL_REQUESTS',
					payload: null,
				},
			},
		})) as Frame | undefined;

		if (!response?.kind) return;

		const kind = response.kind;
		if (!('Response' in kind)) return;

		const respFrame = (kind as Record<string, unknown>)['Response'] as {
			payload?: string;
		};
		if (!respFrame.payload) return;

		let requests: unknown[];
		try {
			requests = JSON.parse(respFrame.payload) as unknown[];
		} catch {
			return;
		}

		if (!Array.isArray(requests) || requests.length === 0) return;

		for (const request of requests) {
			await handlePendingRequest(request);
		}
	} catch {
		// Silently ignore poll errors
	} finally {
		polling = false;
	}
}

async function handlePendingRequest(request: unknown): Promise<void> {
	const req = request as { kind?: Record<string, unknown> };
	const kind = req?.kind;
	if (!kind || !('Request' in kind)) return;

	const reqFrame = (kind as Record<string, unknown>)['Request'] as RequestFrame;

	switch (reqFrame.action) {
		case 'GET_METADATA':
			await handleGetMetadata(reqFrame);
			break;
		case 'GET_ASSETS':
			await handleGetContentData(reqFrame, 'GENERATE_ASSETS');
			break;
		case 'GET_SNAPSHOT':
			await handleGetContentData(reqFrame, 'GENERATE_SNAPSHOT');
			break;
		default: {
			const resp: Frame = {
				kind: {
					Response: {
						id: reqFrame.id,
						action: reqFrame.action,
						payload: JSON.stringify({
							kind: 'Error',
							data: `Unknown action: ${reqFrame.action}`,
						}),
					},
				},
			};
			try {
				await browser.runtime.sendNativeMessage(host, resp);
			} catch {
				/* ignore */
			}
			break;
		}
	}
}

async function handleGetContentData(reqFrame: RequestFrame, messageType: string): Promise<void> {
	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

		if (!activeTab || !activeTab.id) {
			await sendErrorResponse(reqFrame, 'No active tab found');
			return;
		}

		const contentResponse = await sendMessageWithRetry(activeTab.id, { type: messageType });

		const resp: Frame = {
			kind: {
				Response: {
					id: reqFrame.id,
					action: reqFrame.action,
					payload: JSON.stringify(contentResponse),
				},
			},
		};
		await browser.runtime.sendNativeMessage(host, resp);
	} catch (error) {
		try {
			await sendErrorResponse(reqFrame, `Failed to get ${messageType}: ${error}`);
		} catch {
			/* ignore */
		}
	}
}

async function sendErrorResponse(reqFrame: RequestFrame, message: string): Promise<void> {
	const resp: Frame = {
		kind: {
			Response: {
				id: reqFrame.id,
				action: reqFrame.action,
				payload: JSON.stringify({
					kind: 'Error',
					data: message,
				}),
			},
		},
	};
	await browser.runtime.sendNativeMessage(host, resp);
}

async function handleGetMetadata(reqFrame: RequestFrame): Promise<void> {
	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

		if (!activeTab || !activeTab.id) {
			await sendErrorResponse(reqFrame, 'No active tab found');
			return;
		}

		const iconBase64 = await getCurrentTabIcon(activeTab);

		const resp: Frame = {
			kind: {
				Response: {
					id: reqFrame.id,
					action: reqFrame.action,
					payload: JSON.stringify({
						kind: 'NativeMetadata',
						data: {
							url: activeTab.url,
							icon_base64: iconBase64,
						} as NativeMetadata,
					}),
				},
			},
		};
		await browser.runtime.sendNativeMessage(host, resp);
	} catch (error) {
		try {
			await sendErrorResponse(reqFrame, `Failed to get metadata: ${error}`);
		} catch {
			/* ignore */
		}
	}
}
