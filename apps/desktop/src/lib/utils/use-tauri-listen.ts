import { onCleanup } from 'runed';

/**
 * Lifecycle-bound wrapper for Tauri's promise-returning event listeners
 * (e.g. `appWindow.onResized`, `listen()`). Mirrors runed's `useEventListener`
 * shape — call once at component setup, cleanup happens on teardown.
 *
 * The `cancelled` flag guards against the component unmounting *before* the
 * `setup()` promise resolves: the resolved unlisten function is invoked
 * immediately rather than leaking past teardown.
 */
export function useTauriListen(setup: () => Promise<() => void>): void {
	let unlisten: (() => void) | null = null;
	let cancelled = false;

	setup().then(
		(fn) => {
			if (cancelled) fn();
			else unlisten = fn;
		},
		(err) => {
			console.error('useTauriListen: setup failed', err);
		},
	);

	onCleanup(() => {
		cancelled = true;
		unlisten?.();
		unlisten = null;
	});
}
