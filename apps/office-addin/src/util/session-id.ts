const STORAGE_KEY = 'eurora.office.sessionId';

// 32-bit unsigned, stable for the runtime's lifetime so reconnects after
// transient drops re-register under the same `app_pid` the desktop already
// knows. A new runtime (Word reload) gets a fresh id, which the desktop
// treats as a fresh registration.
export function getSessionId(): number {
	const cached = readCached();
	if (cached !== null) return cached;

	const fresh = generate();
	persist(fresh);
	return fresh;
}

function generate(): number {
	const buf = new Uint32Array(1);
	globalThis.crypto.getRandomValues(buf);
	return buf[0]!;
}

function readCached(): number | null {
	try {
		const raw = globalThis.sessionStorage?.getItem(STORAGE_KEY);
		if (raw === null || raw === undefined) return null;
		const parsed = Number.parseInt(raw, 10);
		return Number.isInteger(parsed) && parsed >= 0 ? parsed : null;
	} catch {
		return null;
	}
}

function persist(id: number): void {
	try {
		globalThis.sessionStorage?.setItem(STORAGE_KEY, String(id));
	} catch {
		// sessionStorage unavailable (e.g., file:// in some embeddings) — ok.
	}
}
