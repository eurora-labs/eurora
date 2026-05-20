import browser from 'webextension-polyfill';
import type { Frame, Payload, RequestFrame, ResponseFrame } from '../content/bindings';

/// Error-frame `code` values used by the tab RPCs. Kept in sync with
/// the desktop-side mapping in `eurora-tools::bridge::map_bridge_err`
/// so the bridge contract stays symmetric:
///
/// - `400 BAD_REQUEST`: payload is malformed (missing `tab_id`, wrong
///   shape) **or** a content-script safety-contract violation
///   (`insert_text` against a password field, unknown `field_id`, …).
///   Safety violations carry the LLM-fixable details forward instead
///   of being lumped in with internal handler bugs.
/// - `410 GONE`: `chrome.tabs.sendMessage` rejected — the tab no longer
///   exists or has no content-script listener. Desktop maps this to
///   `ToolError::ContextUnavailable` so the chat layer drops the call
///   instead of retrying within the turn.
/// - `500 INTERNAL`: the content script reached its handler but
///   reported a structured error or returned nothing.
export const CODE_BAD_REQUEST = 400;
export const CODE_TAB_GONE = 410;
export const CODE_CONTENT_ERROR = 500;

/// Sentinel `code` value content scripts use to mark safety-contract
/// violations as user-recoverable (mapped to `400 BAD_REQUEST` by
/// `forwardTabRpc`) rather than internal handler bugs (mapped to
/// `500 CONTENT_ERROR`). Today only `insert_text` emits this.
export const SAFETY_VIOLATION = 'SAFETY_VIOLATION';

/// Forward a tab-targeted RPC frame to the content script identified
/// by the `tab_id` in its payload, returning the typed reply verbatim
/// as a `ResponseFrame`. See the `code` constants above for failure
/// modes.
///
/// `frame.payload` is the bridge protocol's inline JSON value
/// (`Payload`, generated as `unknown`): no `JSON.parse` step — the
/// outer-frame parse already decoded the inline JSON. The payload's
/// flat `{ tab_id, …args }` shape is unpacked here: `tab_id` drives
/// routing, the remaining fields ride alongside `type` in the message
/// the content script receives so handler `parseArgs` lookups succeed.
export async function forwardTabRpc(frame: RequestFrame, messageType: string): Promise<Frame> {
	let tabId: number;
	let args: Record<string, unknown>;
	try {
		tabId = parseTabId(frame.payload);
		args = extractArgs(frame.payload);
	} catch (err) {
		return errorFrame(frame, CODE_BAD_REQUEST, errorMessage(err));
	}

	let reply: unknown;
	try {
		reply = await browser.tabs.sendMessage(tabId, { ...args, type: messageType });
	} catch (err) {
		return errorFrame(frame, CODE_TAB_GONE, `tab ${tabId} unreachable: ${errorMessage(err)}`);
	}

	if (isSafetyViolation(reply)) {
		const detail = typeof reply.data === 'string' ? reply.data : JSON.stringify(reply.data);
		return errorFrame(frame, CODE_BAD_REQUEST, detail);
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
		payload: reply as Payload,
	};
	return { kind: { Response: response } } as Frame;
}

/// Pull every payload field except `tab_id` into a flat record the
/// content script can read as the tool's args. `tab_id` is excluded
/// because it's a transport-layer routing key — the content script
/// neither needs nor wants it in handler arg parsing.
///
/// Throws if `payload` isn't a JSON object. Callers should already
/// have passed it through [`parseTabId`], which performs the same
/// shape check; this helper is paranoid about a malformed `payload`
/// reaching the message-sender even so.
export function extractArgs(payload: Payload | null | undefined): Record<string, unknown> {
	if (payload === null || payload === undefined) {
		return {};
	}
	if (typeof payload !== 'object' || Array.isArray(payload)) {
		throw new Error('payload must be a JSON object');
	}
	const result: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(payload as Record<string, unknown>)) {
		if (key === 'tab_id') continue;
		result[key] = value;
	}
	return result;
}

/// Pull `tab_id` out of an inline payload value. The payload arrives
/// already decoded as a JS value (the outer Frame parse handles the
/// JSON layer), so this is a pure shape check.
export function parseTabId(payload: Payload | null | undefined): number {
	if (payload === null || payload === undefined) {
		throw new Error('missing payload');
	}
	if (typeof payload !== 'object' || !('tab_id' in (payload as Record<string, unknown>))) {
		throw new Error('payload missing tab_id');
	}
	const raw = (payload as { tab_id: unknown }).tab_id;
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

export function isSafetyViolation(
	reply: unknown,
): reply is { kind: 'Error'; code: typeof SAFETY_VIOLATION; data: unknown } {
	return (
		isContentScriptError(reply) &&
		'code' in reply &&
		(reply as { code: unknown }).code === SAFETY_VIOLATION
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
