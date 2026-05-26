import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Frame, Payload, RequestFrame } from '../../content/bindings';

const sendMessageMock = vi.fn();

vi.mock('webextension-polyfill', () => ({
	default: {
		tabs: { sendMessage: sendMessageMock },
	},
}));

let rpc: typeof import('../tab-rpc');

beforeEach(async () => {
	sendMessageMock.mockReset();
	vi.resetModules();
	rpc = await import('../tab-rpc');
});

function requestFrame(payload: Payload | null = { tab_id: 19 } as Payload): RequestFrame {
	return { id: 42, action: 'YOUTUBE_GET_CURRENT_TIMESTAMP', payload };
}

function asError(frame: Frame): { id: number; code: number; message: string } {
	const kind = frame.kind as { Error?: { id: number; code: number; message: string } };
	if (!kind.Error) throw new Error(`expected Error frame, got ${JSON.stringify(frame)}`);
	return kind.Error;
}

function asResponse(frame: Frame): { id: number; action: string; payload: Payload | null } {
	const kind = frame.kind as {
		Response?: { id: number; action: string; payload: Payload | null };
	};
	if (!kind.Response) throw new Error(`expected Response frame, got ${JSON.stringify(frame)}`);
	return kind.Response;
}

describe('parseTabId', () => {
	it('parses integer tab_id', () => {
		expect(rpc.parseTabId({ tab_id: 19 } as Payload)).toBe(19);
	});

	it('rejects missing payload', () => {
		expect(() => rpc.parseTabId(null)).toThrow(/missing payload/);
	});

	it('rejects payload without tab_id', () => {
		expect(() => rpc.parseTabId({} as Payload)).toThrow(/missing tab_id/);
	});

	it('rejects non-integer tab_id', () => {
		expect(() => rpc.parseTabId({ tab_id: '19' } as Payload)).toThrow(/integer/);
		expect(() => rpc.parseTabId({ tab_id: 19.5 } as Payload)).toThrow(/integer/);
	});
});

describe('forwardTabRpc', () => {
	it('returns Response carrying the content-script reply verbatim', async () => {
		const reply = {
			video_id: 'abc123',
			current_time: 12.5,
			duration: 240.0,
			playing: true,
		};
		sendMessageMock.mockResolvedValueOnce(reply);

		const result = await rpc.forwardTabRpc(requestFrame(), 'GET_CURRENT_TIMESTAMP');
		const response = asResponse(result);

		expect(response.id).toBe(42);
		expect(response.action).toBe('YOUTUBE_GET_CURRENT_TIMESTAMP');
		// Payload is the inline JS value — no `JSON.parse` step on the
		// consumer side, since the outer-frame parse already decoded it.
		expect(response.payload).toEqual(reply);
		expect(sendMessageMock).toHaveBeenCalledWith(19, { type: 'GET_CURRENT_TIMESTAMP' });
	});

	it('forwards caller args alongside type, dropping tab_id', async () => {
		sendMessageMock.mockResolvedValueOnce({ matches: [] });

		const frame: RequestFrame = {
			id: 1,
			action: 'WEB_QUERY_SELECTOR',
			payload: {
				tab_id: 19,
				selector: 'textarea',
				limit: 5,
				include: ['text', 'attributes'],
			} as Payload,
		};
		await rpc.forwardTabRpc(frame, 'QUERY_SELECTOR');

		expect(sendMessageMock).toHaveBeenCalledWith(19, {
			selector: 'textarea',
			limit: 5,
			include: ['text', 'attributes'],
			type: 'QUERY_SELECTOR',
		});
	});

	it('refuses to let an arg named "type" override the message type', async () => {
		sendMessageMock.mockResolvedValueOnce({ ok: true });

		const frame: RequestFrame = {
			id: 1,
			action: 'WEB_INSERT_TEXT',
			payload: {
				tab_id: 19,
				field_id: '#x',
				text: 'hello',
				// Hostile arg — the routing key must win.
				type: 'NOT_THE_REAL_TYPE',
			} as Payload,
		};
		await rpc.forwardTabRpc(frame, 'INSERT_TEXT');

		expect(sendMessageMock).toHaveBeenCalledWith(
			19,
			expect.objectContaining({ type: 'INSERT_TEXT', field_id: '#x', text: 'hello' }),
		);
	});

	it('returns 400 for missing payload', async () => {
		const result = await rpc.forwardTabRpc(requestFrame(null), 'GET_CURRENT_TIMESTAMP');
		expect(asError(result).code).toBe(400);
		expect(sendMessageMock).not.toHaveBeenCalled();
	});

	it('returns 410 when sendMessage throws (tab gone)', async () => {
		sendMessageMock.mockRejectedValueOnce(new Error('Receiving end does not exist'));

		const result = await rpc.forwardTabRpc(requestFrame(), 'GET_CURRENT_TIMESTAMP');
		const err = asError(result);
		expect(err.code).toBe(410);
		expect(err.message).toContain('tab 19 unreachable');
	});

	it('returns 500 when the content script reports a structured error', async () => {
		sendMessageMock.mockResolvedValueOnce({ kind: 'Error', data: 'player not ready' });

		const result = await rpc.forwardTabRpc(requestFrame(), 'GET_CURRENT_TIMESTAMP');
		const err = asError(result);
		expect(err.code).toBe(500);
		expect(err.message).toBe('player not ready');
	});

	it('returns 500 when the content script returns no payload', async () => {
		sendMessageMock.mockResolvedValueOnce(undefined);

		const result = await rpc.forwardTabRpc(requestFrame(), 'GET_TRANSCRIPT');
		const err = asError(result);
		expect(err.code).toBe(500);
		expect(err.message).toContain('no payload');
	});

	it('serializes non-string error data into the message', async () => {
		sendMessageMock.mockResolvedValueOnce({ kind: 'Error', data: { code: 1, reason: 'x' } });

		const result = await rpc.forwardTabRpc(requestFrame(), 'GET_CURRENT_TIMESTAMP');
		expect(asError(result).message).toBe('{"code":1,"reason":"x"}');
	});

	it('returns 400 when the content script reports a safety violation', async () => {
		sendMessageMock.mockResolvedValueOnce({
			kind: 'Error',
			code: 'SAFETY_VIOLATION',
			data: 'field_id "input[type=password]" is not a writable text field',
		});

		const result = await rpc.forwardTabRpc(requestFrame(), 'INSERT_TEXT');
		const err = asError(result);
		expect(err.code).toBe(400);
		expect(err.message).toBe('field_id "input[type=password]" is not a writable text field');
	});

	it('serializes non-string safety-violation data into the 400 message', async () => {
		sendMessageMock.mockResolvedValueOnce({
			kind: 'Error',
			code: 'SAFETY_VIOLATION',
			data: { field_id: '#x', reason: 'disabled' },
		});

		const result = await rpc.forwardTabRpc(requestFrame(), 'INSERT_TEXT');
		const err = asError(result);
		expect(err.code).toBe(400);
		expect(err.message).toBe('{"field_id":"#x","reason":"disabled"}');
	});

	it('treats unrelated content-script error codes as 500', async () => {
		sendMessageMock.mockResolvedValueOnce({
			kind: 'Error',
			code: 'something-else',
			data: 'boom',
		});

		const result = await rpc.forwardTabRpc(requestFrame(), 'INSERT_TEXT');
		expect(asError(result).code).toBe(500);
	});
});

describe('extractArgs', () => {
	it('returns {} for null/undefined payloads', () => {
		expect(rpc.extractArgs(null)).toEqual({});
		expect(rpc.extractArgs(undefined)).toEqual({});
	});

	it('returns every non-tab_id field', () => {
		expect(rpc.extractArgs({ tab_id: 1, selector: 'p', limit: 5 } as Payload)).toEqual({
			selector: 'p',
			limit: 5,
		});
	});

	it('throws when payload is not a plain object', () => {
		expect(() => rpc.extractArgs([] as unknown as Payload)).toThrow(/JSON object/);
	});
});

describe('isSafetyViolation', () => {
	it('matches Error frames whose code is the SAFETY_VIOLATION sentinel', () => {
		expect(rpc.isSafetyViolation({ kind: 'Error', code: 'SAFETY_VIOLATION', data: 'x' })).toBe(
			true,
		);
	});

	it('rejects Error frames without a code field', () => {
		expect(rpc.isSafetyViolation({ kind: 'Error', data: 'x' })).toBe(false);
	});

	it('rejects non-Error replies', () => {
		expect(rpc.isSafetyViolation({ kind: 'Response', data: {} })).toBe(false);
		expect(rpc.isSafetyViolation(null)).toBe(false);
		expect(rpc.isSafetyViolation('SAFETY_VIOLATION')).toBe(false);
	});
});
