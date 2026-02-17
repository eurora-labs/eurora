import { type Entry } from './registry';

export function matchSite(host: string, entries: Entry[]): Entry | null {
	// Precompute maps once per activation for O(1) exact / suffix
	const exact = new Map<string, Entry>();
	const suffix: [string, Entry][] = [];
	for (const e of entries) {
		for (const p of e.patterns) {
			if (p.startsWith('*.')) suffix.push([p.slice(2), e]);
			else exact.set(p, e);
		}
	}
	const hit = exact.get(host);
	if (hit) return hit;
	for (const [suf, e] of suffix) {
		if (host === suf || host.endsWith('.' + suf)) return e;
	}
	return null;
}
