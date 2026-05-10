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
 * the latest value is flushed synchronously.
 *
 * Equivalent to React's `useTransition` / `startTransition` pattern: the
 * downstream render is throttled by browser idle scheduling rather than by
 * a fixed clock, so heavy work (markdown reparse, token tree rebuild) only
 * runs when there's spare time.
 */
export class IdleRef<T> {
	#current: T = $state()!;
	#pending: IdleSchedulerHandle | null = null;

	constructor({ source, isLive, timeout = DEFAULT_TIMEOUT_MS }: IdleRefOptions<T>) {
		this.#current = source();

		watch(
			() => [source(), isLive()] as const,
			([next, live]) => {
				if (!live) {
					this.#cancel();
					this.#current = next;
					return;
				}

				this.#cancel();
				this.#pending = scheduleIdle(() => {
					this.#pending = null;
					this.#current = source();
				}, timeout);
			},
		);
	}

	get current(): T {
		return this.#current;
	}

	#cancel() {
		if (this.#pending) {
			this.#pending.cancel();
			this.#pending = null;
		}
	}
}

export function useIdleRef<T>(options: IdleRefOptions<T>): IdleRef<T> {
	return new IdleRef(options);
}
