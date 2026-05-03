import type {
	CancelFrame,
	ErrorFrame,
	EventFrame,
	Frame,
	FrameKind,
	RegisterFrame,
	RequestFrame,
	ResponseFrame,
} from '$lib/shared/bindings';

export function registerFrame(host_pid: number, app_pid: number, app_kind: string): Frame {
	return { kind: { Register: { host_pid, app_pid, app_kind } } };
}

export function responseFrame(id: number, action: string, payload: string | null): Frame {
	return { kind: { Response: { id, action, payload } } };
}

export function errorFrame(
	id: number,
	code: number,
	message: string,
	details: string | null = null,
): Frame {
	return { kind: { Error: { id, code, message, details } } };
}

export function isRegister(kind: FrameKind): kind is { Register: RegisterFrame } {
	return 'Register' in kind;
}

export function isRequest(kind: FrameKind): kind is { Request: RequestFrame } {
	return 'Request' in kind;
}

export function isResponse(kind: FrameKind): kind is { Response: ResponseFrame } {
	return 'Response' in kind;
}

export function isEvent(kind: FrameKind): kind is { Event: EventFrame } {
	return 'Event' in kind;
}

export function isError(kind: FrameKind): kind is { Error: ErrorFrame } {
	return 'Error' in kind;
}

export function isCancel(kind: FrameKind): kind is { Cancel: CancelFrame } {
	return 'Cancel' in kind;
}

// Narrows untyped JSON to a `Frame` shape suitable for dispatch. We only
// validate the discriminator presence; deeper structural validation is the
// desktop's job (it owns the schema).
export function parseFrame(raw: unknown): Frame | null {
	if (typeof raw !== 'object' || raw === null) return null;
	const candidate = raw as { kind?: unknown };
	if (typeof candidate.kind !== 'object' || candidate.kind === null) return null;

	const keys = Object.keys(candidate.kind);
	if (keys.length !== 1) return null;
	const tag = keys[0]!;
	if (
		tag !== 'Request' &&
		tag !== 'Response' &&
		tag !== 'Event' &&
		tag !== 'Error' &&
		tag !== 'Cancel' &&
		tag !== 'Register'
	) {
		return null;
	}
	return candidate as Frame;
}
