import browser from 'webextension-polyfill';
import type { Frame, RequestFrame, ResponseFrame } from '../content/bindings';

/// Error-frame `code` values used by the YouTube tab RPCs. Kept in
/// sync with the desktop-side mapping in
/// `crates/app/euro-tauri/src/tools/youtube.rs` so the bridge contract
/// stays symmetric:
///
/// - `400 BAD_REQUEST`: payload is malformed (missing `tab_id`, bad JSON).
/// - `410 GONE`: `chrome.tabs.sendMessage` rejected — the tab no longer
///   exists or has no content-script listener. Desktop maps this to
///   `ToolError::ContextUnavailable` so the chat layer drops the call
///   instead of retrying within the turn.
/// - `500 INTERNAL`: the content script reached its handler but
///   reported a structured error or returned nothing.
export const CODE_BAD_REQUEST = 400;
export const CODE_TAB_GONE = 410;
export const CODE_CONTENT_ERROR = 500;

/// Forward a tab-targeted RPC frame to the content script identified
/// by the `tab_id` in its payload, returning the typed reply verbatim
/// as a `ResponseFrame`. See [`forwardTabRpc`]'s `code` table for
/// failure modes.
export async function forwardTabRpc(frame: RequestFrame, messageType: string): Promise<Frame> {
	let tabId: number;
	try {
		tabId = parseTabId(frame.payload);
	} catch (err) {
		return errorFrame(frame, CODE_BAD_REQUEST, errorMessage(err));
	}

	let reply: unknown;
	try {
		reply = await browser.tabs.sendMessage(tabId, { type: messageType });
	} catch (err) {
		return errorFrame(frame, CODE_TAB_GONE, `tab ${tabId} unreachable: ${errorMessage(err)}`);
	}

	if (isContentScriptError(reply)) {
		const detail = typeof reply.data === 'string' ? reply.data : JSON.stringify(reply.data);
		return errorFrame(frame, CODE_CONTENT_ERROR, detail);
	}

	if (reply === undefined || reply === null) {
		return errorFrame(
			frame,
			CODE_CONTENT_ERROR,
			`content script returned no payload for ${messageType}`,
		);
	}

	const response: ResponseFrame = {
		id: frame.id,
		action: frame.action,
		payload: JSON.stringify(reply),
	};
	return { kind: { Response: response } } as Frame;
}

export function parseTabId(payload: string | null | undefined): number {
	if (!payload) throw new Error('missing payload');
	let parsed: unknown;
	try {
		parsed = JSON.parse(payload);
	} catch (err) {
		throw new Error(`malformed payload: ${errorMessage(err)}`);
	}
	if (typeof parsed !== 'object' || parsed === null || !('tab_id' in parsed)) {
		throw new Error('payload missing tab_id');
	}
	const raw = (parsed as { tab_id: unknown }).tab_id;
	if (typeof raw !== 'number' || !Number.isInteger(raw)) {
		throw new Error('tab_id must be an integer');
	}
	return raw;
}

export function isContentScriptError(reply: unknown): reply is { kind: 'Error'; data: unknown } {
	return (
		typeof reply === 'object' &&
		reply !== null &&
		'kind' in reply &&
		(reply as { kind: unknown }).kind === 'Error'
	);
}

export function errorFrame(frame: RequestFrame, code: number, message: string): Frame {
	return {
		kind: {
			Error: {
				id: frame.id,
				code,
				message,
				details: null,
			},
		},
	} as Frame;
}

function errorMessage(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}
