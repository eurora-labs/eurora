import { watch } from 'runed';

export interface IdleRefOptions<T> {
	source: () => T;
	isLive: () => boolean;
	/**
	 * Maximum delay before the deferred update is forced through, even if the
	 * browser never becomes idle. Mirrors `requestIdleCallback`'s `timeout`
	 * option. Defaults to 100ms — short enough to feel live, long enough that
	 * the browser can paint between updates under load.
	 */
	timeout?: number;
}

interface IdleSchedulerHandle {
	cancel(): void;
}

const DEFAULT_TIMEOUT_MS = 100;

function scheduleIdle(callback: () => void, timeout: number): IdleSchedulerHandle {
	if (typeof window === 'undefined') {
		callback();
		return { cancel() {} };
	}

	if (typeof window.requestIdleCallback === 'function') {
		const handle = window.requestIdleCallback(() => callback(), { timeout });
		return {
			cancel() {
				window.cancelIdleCallback?.(handle);
			},
		};
	}

	const handle = window.setTimeout(callback, timeout);
	return {
		cancel() {
			window.clearTimeout(handle);
		},
	};
}

/**
 * Reactive value that mirrors `source` but defers updates to browser idle
 * time while `isLive` is true. New `source` values supersede pending updates
 * — only the latest snapshot is ever committed. When `isLive` flips false,
 * the latest value is committed synchronously.
 *
 * Equivalent to React's `useTransition` / `startTransition` pattern: the
 * downstream render is throttled by browser idle scheduling rather than by
 * a fixed clock, so heavy work (markdown reparse, token tree rebuild) only
 * runs when there's spare time.
 */
export class IdleRef<T> {
	#current: T | undefined = $state();

	constructor({ source, isLive, timeout = DEFAULT_TIMEOUT_MS }: IdleRefOptions<T>) {
		this.#current = source();

		// `watch` re-runs the effect body on each `source`/`isLive` change.
		// The cleanup function we return is invoked both before the next run
		// AND on host teardown — so cancelling the pending idle callback
		// covers supersession and unmount with the same code path. No
		// long-lived `#pending` state is needed.
		watch(
			() => [source(), isLive()] as const,
			([next, live]) => {
				if (!live) {
					this.#current = next;
					return;
				}

				const handle = scheduleIdle(() => {
					this.#current = source();
				}, timeout);

				return () => handle.cancel();
			},
			{ lazy: true },
		);
	}

	get current(): T {
		// The constructor seeds `#current` before any read can happen, so the
		// observable type is always `T` even though the field is technically
		// `T | undefined` until the first assignment.
		return this.#current as T;
	}
}

export function useIdleRef<T>(options: IdleRefOptions<T>): IdleRef<T> {
	return new IdleRef(options);
}
