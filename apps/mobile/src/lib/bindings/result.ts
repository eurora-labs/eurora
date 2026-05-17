/**
 * Tagged envelope every tauri-specta `Result<T, E>` command returns. Plain
 * (non-`Result`) commands return `Promise<T>` and never use this shape.
 */
export type CommandResult<T, E = string> =
	| { status: 'ok'; data: T }
	| { status: 'error'; error: E };

/**
 * `Error` subclass thrown by [`unwrap`]. The original typed error payload
 * is preserved on `cause` so call sites that want to branch on the
 * underlying variant (`if (e.cause?.type === 'NotAuthenticated')`) still
 * can, while incidental `${error}` interpolation in toast / log strings
 * renders the formatted message via `Error.toString()` rather than
 * `[object Object]`.
 */
export class CommandError<E = unknown> extends Error {
	readonly cause: E;

	constructor(cause: E) {
		super(formatErrorMessage(cause));
		this.name = 'CommandError';
		this.cause = cause;
	}
}

/**
 * Unwrap a tauri-specta typed-error result. Throws a [`CommandError`]
 * carrying the original typed error on `cause`. Call sites that want to
 * render the error inline should branch on `result.status === 'error'`
 * directly instead of unwrapping.
 *
 * Rust commands whose signature is *not* a `Result` are emitted as plain
 * `Promise<T>` and must NOT be passed through this helper.
 */
export function unwrap<T, E>(result: CommandResult<T, E>): T {
	if (result.status === 'error') throw new CommandError(result.error);
	return result.data;
}

/**
 * Best-effort string rendering for an arbitrary command error. Strings
 * pass through; tauri-specta tagged errors (`{ type, data }`) prefer the
 * `data` payload and fall back to the variant name; anything else goes
 * through `String(...)`.
 */
function formatErrorMessage(error: unknown): string {
	if (typeof error === 'string') return error;
	if (error && typeof error === 'object') {
		const e = error as { type?: unknown; data?: unknown };
		if (typeof e.data === 'string') return e.data;
		if (typeof e.type === 'string') return e.type;
	}
	return String(error);
}
