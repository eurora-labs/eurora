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
});
