import { isRequest, parseFrame, registerFrame } from '$lib/bridge/frames';
import * as log from '$lib/util/log';
import { getSessionId } from '$lib/util/session-id';
import type { Frame, RequestFrame } from '$lib/shared/bindings';

const DEFAULT_URL = 'ws://127.0.0.1:1431/bridge';
const DEFAULT_APP_KIND = 'microsoft-word';

export interface BackoffConfig {
	baseMs: number;
	maxMs: number;
	factor: number;
	jitterRatio: number;
}

export const DEFAULT_BACKOFF: BackoffConfig = {
	baseMs: 1_000,
	maxMs: 30_000,
	factor: 2,
	jitterRatio: 0.2,
};

export interface BridgeClientOptions {
	dispatch: (req: RequestFrame) => Promise<Frame>;
	url?: string;
	appKind?: string;
	hostPid?: number;
	appPid?: number;
	backoff?: BackoffConfig;
	// Test seams. Production code uses globals.
	webSocketCtor?: WebSocketLike;
	setTimeoutFn?: (fn: () => void, ms: number) => unknown;
	clearTimeoutFn?: (handle: unknown) => void;
	randomFn?: () => number;
}

export interface BridgeClient {
	start(): void;
	stop(): void;
	readonly state: BridgeState;
}

export type BridgeState = 'idle' | 'connecting' | 'open' | 'reconnecting' | 'stopped';

export type WebSocketLike = new (url: string) => WebSocket;

interface InternalState {
	state: BridgeState;
	socket: WebSocket | null;
	reconnectHandle: unknown;
	attempt: number;
}

export function startBridgeClient(options: BridgeClientOptions): BridgeClient {
	const url = options.url ?? DEFAULT_URL;
	const appKind = options.appKind ?? DEFAULT_APP_KIND;
	const hostPid = options.hostPid ?? 0;
	const appPid = options.appPid ?? getSessionId();
	const backoff = options.backoff ?? DEFAULT_BACKOFF;
	const WSCtor = options.webSocketCtor ?? (globalThis.WebSocket as unknown as WebSocketLike);
	const setTimeoutFn: (fn: () => void, ms: number) => unknown =
		options.setTimeoutFn ?? ((fn, ms) => globalThis.setTimeout(fn, ms));
	const clearTimeoutFn: (handle: unknown) => void =
		options.clearTimeoutFn ?? ((handle) => globalThis.clearTimeout(handle as never));
	const random = options.randomFn ?? Math.random;

	const internal: InternalState = {
		state: 'idle',
		socket: null,
		reconnectHandle: null,
		attempt: 0,
	};

	function connect(): void {
		if (internal.state === 'stopped') return;
		if (internal.socket !== null) return;

		internal.state = 'connecting';
		log.info('connecting to', url, 'attempt', internal.attempt + 1);

		let socket: WebSocket;
		try {
			socket = new WSCtor(url);
		} catch (e) {
			log.error('WebSocket construction failed', e);
			scheduleReconnect();
			return;
		}
		internal.socket = socket;

		socket.addEventListener('open', () => {
			if (internal.socket !== socket) return;
			log.info('connected, registering');
			internal.state = 'open';
			internal.attempt = 0;
			send(registerFrame(hostPid, appPid, appKind));
		});

		socket.addEventListener('message', (ev) => {
			if (internal.socket !== socket) return;
			void onMessage(ev);
		});

		socket.addEventListener('error', (ev) => {
			log.warn('socket error', ev);
		});

		socket.addEventListener('close', (ev) => {
			if (internal.socket !== socket) return;
			log.info('socket closed', ev.code, ev.reason);
			internal.socket = null;
			if (internal.state === 'stopped') return;
			scheduleReconnect();
		});
	}

	function scheduleReconnect(): void {
		if (internal.state === 'stopped') return;
		internal.state = 'reconnecting';
		const delay = nextDelay(internal.attempt, backoff, random);
		internal.attempt += 1;
		log.info('reconnect in', Math.round(delay), 'ms');
		internal.reconnectHandle = setTimeoutFn(() => {
			internal.reconnectHandle = null;
			connect();
		}, delay);
	}

	async function onMessage(ev: MessageEvent): Promise<void> {
		const data = ev.data;
		if (typeof data !== 'string') {
			log.warn('non-string message dropped');
			return;
		}
		let parsed: unknown;
		try {
			parsed = JSON.parse(data);
		} catch (e) {
			log.warn('JSON parse failed', e);
			return;
		}
		const frame = parseFrame(parsed);
		if (frame === null) {
			log.warn('unrecognized frame shape', parsed);
			return;
		}
		const kind = frame.kind;
		if (isRequest(kind)) {
			const response = await options.dispatch(kind.Request);
			send(response);
			return;
		}
		// Other variants are not expected client-bound. The Office add-in
		// never initiates Requests or emits Events, so the desktop should
		// only ever send us Requests. Log and drop for visibility.
		log.warn('dropping non-Request frame', Object.keys(kind)[0]);
	}

	function send(frame: Frame): void {
		const socket = internal.socket;
		if (socket === null || socket.readyState !== WebSocket.OPEN) {
			log.warn('send dropped: socket not open');
			return;
		}
		try {
			socket.send(JSON.stringify(frame));
		} catch (e) {
			log.error('send failed', e);
		}
	}

	function start(): void {
		if (internal.state !== 'idle' && internal.state !== 'stopped') return;
		internal.state = 'idle';
		internal.attempt = 0;
		connect();
	}

	function stop(): void {
		internal.state = 'stopped';
		if (internal.reconnectHandle !== null) {
			clearTimeoutFn(internal.reconnectHandle);
			internal.reconnectHandle = null;
		}
		if (internal.socket !== null) {
			try {
				internal.socket.close(1000, 'client stopping');
			} catch (e) {
				log.warn('close threw', e);
			}
			internal.socket = null;
		}
	}

	start();

	return {
		start,
		stop,
		get state(): BridgeState {
			return internal.state;
		},
	};
}

export function nextDelay(
	attempt: number,
	cfg: BackoffConfig,
	random: () => number = Math.random,
): number {
	const exp = Math.min(cfg.maxMs, cfg.baseMs * Math.pow(cfg.factor, attempt));
	const jitter = exp * cfg.jitterRatio * (random() * 2 - 1);
	return Math.max(0, exp + jitter);
}
