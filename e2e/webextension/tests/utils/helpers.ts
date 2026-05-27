import { expect, type Page, type Worker } from '@playwright/test';
import type {
	ContextResponse,
	InvokeResponse,
	ListToolsResponse,
	WireToolDescriptor,
} from './types.ts';

/// Per-process monotonic `call_id`. The extension keys its in-flight
/// AbortController map by id, so reuse would collide if two specs ran
/// in the same worker and the second one cancelled by id; the global
/// counter makes ids unique across the whole test process.
let nextCallId = 1;

export async function waitForBootstrap(page: Page) {
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-ready', '1');
}

export async function waitForSiteMounted(page: Page, siteId: string) {
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-mounted', '1');
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-site', siteId);
}

/// Send `LIST_TOOLS` to the active tab's content script and return the
/// advertised descriptors. The bridge contract guarantees this returns
/// the current per-page tool surface — re-evaluated by the watcher on
/// every call, so SPA navigations don't require a content-script
/// reload.
export async function listTools(sw: Worker): Promise<WireToolDescriptor[]> {
	const reply = await sendToActiveTab<ListToolsResponse>(sw, { type: 'LIST_TOOLS' });
	return reply.tools;
}

/// Send `GET_CONTEXT` to the active tab and return the watcher's
/// per-page summary blocks.
export async function getContext(sw: Worker): Promise<ContextResponse> {
	return await sendToActiveTab<ContextResponse>(sw, { type: 'GET_CONTEXT' });
}

/// Send `INVOKE_TOOL` to the active tab and unwrap the `{ ok } | { err }`
/// envelope. On success returns the tool's `ok` value typed as `T`; on
/// failure throws a `ToolInvocationError` carrying the structured
/// `ToolErrorWire` so callers can `expect(...).toThrow(...)`-pattern
/// against the typed error.
export async function invokeTool<T>(
	sw: Worker,
	name: string,
	args: Record<string, unknown> = {},
): Promise<T> {
	const callId = nextCallId++;
	const reply = await sendToActiveTab<InvokeResponse>(sw, {
		type: 'INVOKE_TOOL',
		call_id: callId,
		name,
		arguments: args,
	});
	if ('err' in reply) {
		throw new ToolInvocationError(name, reply.err);
	}
	return reply.ok as T;
}

/// Raw `INVOKE_TOOL` variant for error-path specs that need to assert
/// against the structured `ToolErrorWire` envelope without throwing.
/// Most callers should prefer `invokeTool`.
export async function invokeToolRaw(
	sw: Worker,
	name: string,
	args: unknown = {},
): Promise<InvokeResponse> {
	const callId = nextCallId++;
	return await sendToActiveTab<InvokeResponse>(sw, {
		type: 'INVOKE_TOOL',
		call_id: callId,
		name,
		arguments: args,
	});
}

/// Cancel an in-flight tool call. Idempotent — the extension responds
/// with `{}` whether or not the id is currently registered.
export async function cancelTool(sw: Worker, callId: number): Promise<void> {
	await sendToActiveTab<Record<string, never>>(sw, {
		type: 'CANCEL_TOOL',
		call_id: callId,
	});
}

/// Fetch the current `LIST_TOOLS` set and return the descriptor for
/// `name`, throwing a descriptive error if it isn't advertised. Used
/// when a spec needs both presence-pinning and per-tool descriptor
/// assertions (e.g. confirming a tool's `timeout_ms` or `source.kind`).
export async function requireTool(sw: Worker, name: string): Promise<WireToolDescriptor> {
	const tools = await listTools(sw);
	const found = tools.find((t) => t.name === name);
	if (!found) {
		const advertised = tools.map((t) => t.name).join(', ');
		throw new Error(`tool ${name} not advertised. Available: ${advertised}`);
	}
	return found;
}

/// The next `call_id` the helpers will use. Specs that want to assert
/// behavior against an in-flight call (e.g. cancel via `cancelTool`)
/// can capture this before invoking. Returns the value WITHOUT
/// consuming it — use `invokeTool` to actually advance the counter.
export function peekNextCallId(): number {
	return nextCallId;
}

export class ToolInvocationError extends Error {
	constructor(
		readonly tool: string,
		readonly wire: Extract<InvokeResponse, { err: unknown }>['err'],
	) {
		super(
			`tool ${tool} failed: ${wire.kind}${'message' in wire && wire.message ? ` — ${wire.message}` : ''}`,
		);
		this.name = 'ToolInvocationError';
	}
}

interface ToolMessage {
	type: 'LIST_TOOLS' | 'GET_CONTEXT' | 'INVOKE_TOOL' | 'CANCEL_TOOL';
	call_id?: number;
	name?: string;
	arguments?: unknown;
}

async function sendToActiveTab<T>(sw: Worker, message: ToolMessage): Promise<T> {
	return (await sw.evaluate(
		async (msg) => {
			const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
			if (!tab?.id) {
				throw new Error('no active tab in current window');
			}
			return (await chrome.tabs.sendMessage(tab.id, msg)) as unknown;
		},
		message as unknown as Record<string, unknown>,
	)) as T;
}
