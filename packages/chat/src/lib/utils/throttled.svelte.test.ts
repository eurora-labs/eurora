import { IdleRef } from '$lib/utils/throttled.svelte.js';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * `IdleRef` registers a `watch` effect, which only runs inside an effect
 * root. Each test sets one up, exercises the ref, and tears it down at
 * the end so any pending idle callback gets cancelled.
 *
 * `requestIdleCallback` isn't implemented in jsdom; the helper falls back
 * to `setTimeout`. We swap in fake timers so we can step time precisely.
 */

interface Harness<T> {
	ref: IdleRef<T>;
	dispose: () => void;
}

function withRoot<T>(build: () => IdleRef<T>): Harness<T> {
	let ref: IdleRef<T> | undefined;
	let buildError: unknown;
	const dispose = $effect.root(() => {
		try {
			ref = build();
		} catch (err) {
			buildError = err;
		}
	});
	if (buildError) {
		dispose();
		throw buildError;
	}
	if (!ref) {
		dispose();
		throw new Error('IdleRef build did not run synchronously inside $effect.root');
	}
	flushSync();
	return { ref, dispose };
}

describe('IdleRef', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('captures the initial source value synchronously', () => {
		const value = 'hello';
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => false,
				}),
		);
		try {
			expect(ref.current).toBe('hello');
		} finally {
			dispose();
		}
	});

	it('commits updates synchronously while not live', () => {
		let value = $state('a');
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => false,
				}),
		);
		try {
			value = 'b';
			flushSync();
			expect(ref.current).toBe('b');
		} finally {
			dispose();
		}
	});

	it('defers updates until the idle deadline while live', () => {
		let value = $state('chunk-0');
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => true,
					timeout: 50,
				}),
		);
		try {
			expect(ref.current).toBe('chunk-0');

			value = 'chunk-1';
			flushSync();
			// Schedule fired but the timer hasn't elapsed.
			expect(ref.current).toBe('chunk-0');

			vi.advanceTimersByTime(50);
			flushSync();
			expect(ref.current).toBe('chunk-1');
		} finally {
			dispose();
		}
	});

	it('supersedes pending updates with the latest source value', () => {
		let value = $state('chunk-0');
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => true,
					timeout: 50,
				}),
		);
		try {
			value = 'chunk-1';
			flushSync();
			value = 'chunk-2';
			flushSync();
			value = 'chunk-3';
			flushSync();

			// Still on the seed because no timer has elapsed.
			expect(ref.current).toBe('chunk-0');

			vi.advanceTimersByTime(50);
			flushSync();
			// Only the latest snapshot lands.
			expect(ref.current).toBe('chunk-3');
		} finally {
			dispose();
		}
	});

	it('flushes the latest value when isLive flips false', () => {
		let value = $state('chunk-0');
		let live = $state(true);
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => live,
					timeout: 1000,
				}),
		);
		try {
			value = 'chunk-final';
			flushSync();
			// Pending; not yet committed.
			expect(ref.current).toBe('chunk-0');

			live = false;
			flushSync();
			// Sync flush bypasses the idle scheduler.
			expect(ref.current).toBe('chunk-final');
		} finally {
			dispose();
		}
	});

	it('cancels pending updates on teardown', () => {
		const clearSpy = vi.spyOn(globalThis, 'clearTimeout');
		let value = $state('chunk-0');
		const { ref, dispose } = withRoot(
			() =>
				new IdleRef<string>({
					source: () => value,
					isLive: () => true,
					timeout: 1000,
				}),
		);
		try {
			value = 'chunk-1';
			flushSync();
			expect(ref.current).toBe('chunk-0');
		} finally {
			dispose();
		}
		expect(clearSpy).toHaveBeenCalled();

		// Advance past the deadline; the cancelled callback must not fire.
		vi.advanceTimersByTime(2000);
		expect(ref.current).toBe('chunk-0');
	});
});
