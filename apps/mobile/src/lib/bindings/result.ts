/**
 * Unwrap a tauri-specta typed-error result. Every Rust command returning
 * `Result<T, E>` (including `Result<T, String>`) is emitted as
 * `Promise<{ status: 'ok'; data: T } | { status: 'error'; error: E }>` so
 * the caller can branch on the discriminant. Call sites that want to
 * bubble the error up to the global toast/error boundary throw it via
 * this helper; sites that want to render the error inline should branch
 * on `result.status === 'error'` directly instead of unwrapping.
 *
 * Rust commands whose signature is *not* a `Result` are emitted as plain
 * `Promise<T>` and must NOT be passed through this helper.
 */
export function unwrap<T, E>(result: { status: 'ok'; data: T } | { status: 'error'; error: E }): T {
	if (result.status === 'error') throw result.error;
	return result.data;
}
