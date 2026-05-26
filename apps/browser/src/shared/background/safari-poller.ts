import { resolveFaviconBase64 } from './favicon';
import { isSafari } from './util';
import browser from 'webextension-polyfill';
import type { Frame, NativeMetadata, Payload, RequestFrame } from '../content/bindings';

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
			payload?: Payload | null;
		};
		// The inline payload is the bridge's `Payload` (TypeScript `unknown`)
		// — already decoded by the outer-frame parse. The Safari poll
		// contract is a list of pending `Request` frames; reject anything
		// else as a shape mismatch.
		if (!Array.isArray(respFrame.payload)) return;
		const requests: unknown[] = respFrame.payload;

		if (requests.length === 0) return;

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
		default: {
			const resp: Frame = {
				kind: {
					Response: {
						id: reqFrame.id,
						action: reqFrame.action,
						payload: {
							kind: 'Error',
							data: `Unknown action: ${reqFrame.action}`,
						} as Payload,
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

async function sendErrorResponse(reqFrame: RequestFrame, message: string): Promise<void> {
	const resp: Frame = {
		kind: {
			Response: {
				id: reqFrame.id,
				action: reqFrame.action,
				payload: {
					kind: 'Error',
					data: message,
				} as Payload,
			},
		},
	};
	await browser.runtime.sendNativeMessage(host, resp);
}

async function handleGetMetadata(reqFrame: RequestFrame): Promise<void> {
	try {
		const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });

		if (!activeTab || activeTab.id === undefined) {
			await sendErrorResponse(reqFrame, 'No active tab found');
			return;
		}

		const iconBase64 = await resolveFaviconBase64(activeTab);

		const resp: Frame = {
			kind: {
				Response: {
					id: reqFrame.id,
					action: reqFrame.action,
					payload: {
						kind: 'NativeMetadata',
						data: {
							tab_id: activeTab.id,
							url: activeTab.url ?? null,
							icon_base64: iconBase64,
							title: activeTab.title ?? null,
						} as NativeMetadata,
					} as Payload,
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
