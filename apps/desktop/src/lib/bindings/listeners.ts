/**
 * Collects pending tauri-specta `events.*.listen(...)` promises and tears
 * them all down on `destroy`. `destroy` awaits every pending registration
 * before firing the unlisten callbacks, which closes a one-frame race in
 * the previous ad-hoc patterns where a component could navigate away
 * before `listen` resolved and miss the unlisten.
 *
 * Failed registrations (rejected promises) are ignored — call sites that
 * want to surface registration errors should `.catch` on the promise
 * before handing it to `add`.
 */
export class ListenerBag {
	#pending: Promise<() => void>[] = [];

	add(p: Promise<() => void>): void {
		this.#pending.push(p);
	}

	async destroy(): Promise<void> {
		const settled = await Promise.allSettled(this.#pending);
		this.#pending.length = 0;
		for (const r of settled) {
			if (r.status === 'fulfilled') r.value();
		}
	}
}
