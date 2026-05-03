import { DEFAULT_BACKOFF, nextDelay, startBridgeClient } from '$lib/bridge/client';
import { responseFrame } from '$lib/bridge/frames';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

describe('nextDelay', () => {
	const cfg = { ...DEFAULT_BACKOFF, jitterRatio: 0 };

	it('grows exponentially up to the cap', () => {
		expect(nextDelay(0, cfg)).toBe(1_000);
		expect(nextDelay(1, cfg)).toBe(2_000);
		expect(nextDelay(2, cfg)).toBe(4_000);
		expect(nextDelay(10, cfg)).toBe(30_000);
	});

	it('applies symmetric jitter from the supplied PRNG', () => {
		const jitterCfg = { ...DEFAULT_BACKOFF, jitterRatio: 0.5 };
		expect(nextDelay(0, jitterCfg, () => 0)).toBe(500);
		expect(nextDelay(0, jitterCfg, () => 1)).toBe(1500);
		expect(nextDelay(0, jitterCfg, () => 0.5)).toBe(1000);
	});

	it('never returns a negative delay', () => {
		const big = { ...DEFAULT_BACKOFF, jitterRatio: 5 };
		expect(nextDelay(0, big, () => 0)).toBeGreaterThanOrEqual(0);
	});
});

interface FakeListenerMap {
	open: Array<() => void>;
	message: Array<(ev: MessageEvent) => void>;
	error: Array<(ev: Event) => void>;
	close: Array<(ev: CloseEvent) => void>;
}

class FakeWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;
	static instances: FakeWebSocket[] = [];

	readyState: number = FakeWebSocket.CONNECTING;
	sent: string[] = [];
	closed: { code?: number; reason?: string } | null = null;
	readonly url: string;
	private readonly listeners: FakeListenerMap = {
		open: [],
		message: [],
		error: [],
		close: [],
	};

	constructor(url: string) {
		this.url = url;
		FakeWebSocket.instances.push(this);
	}

	addEventListener<K extends keyof FakeListenerMap>(
		type: K,
		fn: FakeListenerMap[K][number],
	): void {
		(this.listeners[type] as unknown[]).push(fn);
	}

	send(data: string): void {
		this.sent.push(data);
	}

	close(code?: number, reason?: string): void {
		this.readyState = FakeWebSocket.CLOSED;
		this.closed = { code, reason };
	}

	emitOpen(): void {
		this.readyState = FakeWebSocket.OPEN;
		for (const fn of this.listeners.open) fn();
	}

	emitMessage(data: string): void {
		const ev = { data } as MessageEvent;
		for (const fn of this.listeners.message) fn(ev);
	}

	emitClose(code = 1006, reason = 'abnormal'): void {
		this.readyState = FakeWebSocket.CLOSED;
		const ev = { code, reason } as CloseEvent;
		for (const fn of this.listeners.close) fn(ev);
	}
}

interface ScheduledTimer {
	fn: () => void;
	ms: number;
}

interface Harness {
	client: ReturnType<typeof startBridgeClient>;
	timers: ScheduledTimer[];
	clearTimeoutFn: ReturnType<typeof vi.fn>;
	dispatch: ReturnType<typeof vi.fn>;
}

function makeClient(dispatch?: ReturnType<typeof vi.fn>): Harness {
	const dispatchFn = dispatch ?? vi.fn().mockResolvedValue(responseFrame(1, 'X', null));
	const timers: ScheduledTimer[] = [];
	function setTimeoutFn(fn: () => void, ms: number): unknown {
		timers.push({ fn, ms });
		return timers.length;
	}
	const clearTimeoutFn = vi.fn();
	const client = startBridgeClient({
		dispatch: dispatchFn,
		url: 'ws://test/bridge',
		appKind: 'microsoft-word',
		hostPid: 0,
		appPid: 99,
		webSocketCtor: FakeWebSocket as unknown as new (url: string) => WebSocket,
		setTimeoutFn,
		clearTimeoutFn,
		randomFn: () => 0.5,
		backoff: { ...DEFAULT_BACKOFF, jitterRatio: 0 },
	});
	return { client, timers, clearTimeoutFn, dispatch: dispatchFn };
}

describe('startBridgeClient', () => {
	beforeEach(() => {
		FakeWebSocket.instances = [];
		(globalThis as unknown as { WebSocket: unknown }).WebSocket = FakeWebSocket;
	});

	afterEach(() => {
		delete (globalThis as unknown as { WebSocket?: unknown }).WebSocket;
	});

	it('opens a socket on start and registers on open', () => {
		const { client } = makeClient();
		expect(FakeWebSocket.instances).toHaveLength(1);
		const sock = FakeWebSocket.instances[0]!;
		sock.emitOpen();
		expect(client.state).toBe('open');
		expect(sock.sent).toHaveLength(1);
		expect(JSON.parse(sock.sent[0]!)).toEqual({
			kind: { Register: { host_pid: 0, app_pid: 99, app_kind: 'microsoft-word' } },
		});
	});

	it('dispatches incoming Request frames and writes the Response back', async () => {
		const dispatch = vi
			.fn()
			.mockResolvedValue(responseFrame(7, 'GET_ASSETS', JSON.stringify({ ok: true })));
		makeClient(dispatch);
		const sock = FakeWebSocket.instances[0]!;
		sock.emitOpen();
		sock.emitMessage(
			JSON.stringify({
				kind: { Request: { id: 7, action: 'GET_ASSETS', payload: null } },
			}),
		);

		await vi.waitFor(() => {
			expect(dispatch).toHaveBeenCalled();
		});
		expect(dispatch).toHaveBeenCalledWith({ id: 7, action: 'GET_ASSETS', payload: null });

		await vi.waitFor(() => {
			expect(sock.sent).toHaveLength(2);
		});
		expect(JSON.parse(sock.sent[1]!)).toEqual({
			kind: {
				Response: { id: 7, action: 'GET_ASSETS', payload: JSON.stringify({ ok: true }) },
			},
		});
	});

	it('schedules a reconnect after the socket closes', () => {
		const { client, timers } = makeClient();
		const sock = FakeWebSocket.instances[0]!;
		sock.emitOpen();
		sock.emitClose();

		expect(client.state).toBe('reconnecting');
		expect(timers).toHaveLength(1);
		expect(timers[0]!.ms).toBe(1_000);

		timers[0]!.fn();
		expect(FakeWebSocket.instances).toHaveLength(2);
	});

	it('uses exponential backoff across consecutive failures', () => {
		const { timers } = makeClient();
		FakeWebSocket.instances[0]!.emitClose();
		expect(timers[0]!.ms).toBe(1_000);
		timers[0]!.fn();
		FakeWebSocket.instances[1]!.emitClose();
		expect(timers[1]!.ms).toBe(2_000);
		timers[1]!.fn();
		FakeWebSocket.instances[2]!.emitClose();
		expect(timers[2]!.ms).toBe(4_000);
	});

	it('resets backoff after a successful registration', () => {
		const { timers } = makeClient();
		FakeWebSocket.instances[0]!.emitClose();
		timers[0]!.fn();
		FakeWebSocket.instances[1]!.emitOpen();
		FakeWebSocket.instances[1]!.emitClose();
		expect(timers[1]!.ms).toBe(1_000);
	});

	it('stop() halts reconnect and closes the live socket', () => {
		const { client, timers, clearTimeoutFn } = makeClient();
		const sock = FakeWebSocket.instances[0]!;
		sock.emitOpen();
		sock.emitClose();
		expect(timers).toHaveLength(1);
		client.stop();
		expect(clearTimeoutFn).toHaveBeenCalled();
		expect(client.state).toBe('stopped');
	});

	it('drops malformed frames without crashing', () => {
		makeClient();
		const sock = FakeWebSocket.instances[0]!;
		sock.emitOpen();
		sock.emitMessage('not json');
		sock.emitMessage(JSON.stringify({ no: 'kind' }));
		sock.emitMessage(JSON.stringify({ kind: { Bogus: {} } }));
		expect(sock.sent).toHaveLength(1);
	});
});
