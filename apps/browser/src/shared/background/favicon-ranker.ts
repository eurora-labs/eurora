export type IconSource = 'dom' | 'tab' | 'origin';

export interface IconCandidate {
	href: string;
	rel: string;
	type: string;
	size: number;
	source: IconSource;
	order: number;
}

export interface IconLinkRecord {
	href: string;
	rel: string;
	type: string;
	sizes: string;
}

const UNSUPPORTED_SCHEMES = [
	'chrome:',
	'chrome-extension:',
	'edge:',
	'about:',
	'moz-extension:',
	'brave:',
	'file:',
];

const SVG_SIZE = Number.POSITIVE_INFINITY;
const TARGET_SIZE = 64;

export function isSupportedScheme(url: string): boolean {
	const lower = url.toLowerCase();
	return !UNSUPPORTED_SCHEMES.some((scheme) => lower.startsWith(scheme));
}

export function parseSizes(raw: string | null | undefined, type: string): number {
	if (type === 'image/svg+xml') return SVG_SIZE;
	if (!raw) return 0;

	const tokens = raw.trim().toLowerCase().split(/\s+/);
	if (tokens.includes('any')) return SVG_SIZE;

	let largest = 0;
	for (const token of tokens) {
		const match = token.match(/^(\d+)x(\d+)$/);
		if (!match) continue;
		const dim = Math.min(parseInt(match[1], 10), parseInt(match[2], 10));
		if (dim > largest) largest = dim;
	}
	return largest;
}

export function isIconRel(rel: string): boolean {
	const r = rel.toLowerCase();
	return (
		r === 'icon' ||
		r === 'shortcut icon' ||
		r.includes('apple-touch-icon') ||
		r === 'mask-icon' ||
		r === 'fluid-icon'
	);
}

export function typeRank(type: string, href: string): number {
	const ext = href.toLowerCase().split('?')[0].split('#')[0].split('.').pop() ?? '';
	const t = type.toLowerCase();
	if (t === 'image/png' || ext === 'png') return 0;
	if (t === 'image/svg+xml' || ext === 'svg') return 1;
	if (t === 'image/x-icon' || t === 'image/vnd.microsoft.icon' || ext === 'ico') return 2;
	if (t.startsWith('image/')) return 3;
	return 4;
}

export function sourceRank(source: IconSource): number {
	switch (source) {
		case 'dom':
			return 0;
		case 'tab':
			return 1;
		case 'origin':
			return 2;
	}
}

/**
 * Picks the best icon from a candidate list. Pure function, easy to test.
 *
 * Ranking (lower score is better):
 *   1. SVG/`sizes="any"` wins outright (vector scales without loss).
 *   2. Among raster, distance from TARGET_SIZE; ties broken by type then source order.
 *   3. PNG > ICO > others (codec robustness).
 *   4. DOM candidates beat the tab-supplied URL when same size, since the DOM
 *      reflects what the page actually declared while `favIconUrl` can lag.
 */
export function rankCandidates(candidates: IconCandidate[]): IconCandidate[] {
	const usable = candidates.filter((c) => c.href && isSupportedScheme(c.href));

	return [...usable].sort((a, b) => {
		const aSvg = a.size === SVG_SIZE;
		const bSvg = b.size === SVG_SIZE;
		if (aSvg !== bSvg) return aSvg ? -1 : 1;

		if (!aSvg) {
			const aDist = a.size > 0 ? Math.abs(a.size - TARGET_SIZE) : 1024;
			const bDist = b.size > 0 ? Math.abs(b.size - TARGET_SIZE) : 1024;
			if (aDist !== bDist) return aDist - bDist;
		}

		const aType = typeRank(a.type, a.href);
		const bType = typeRank(b.type, b.href);
		if (aType !== bType) return aType - bType;

		const aSrc = sourceRank(a.source);
		const bSrc = sourceRank(b.source);
		if (aSrc !== bSrc) return aSrc - bSrc;

		return a.order - b.order;
	});
}

export function collectIconCandidatesFromLinks(
	entries: ReadonlyArray<IconLinkRecord>,
): IconCandidate[] {
	const candidates: IconCandidate[] = [];
	entries.forEach((entry, idx) => {
		if (!entry || typeof entry.href !== 'string' || !entry.href) return;
		if (!isIconRel(entry.rel ?? '')) return;
		candidates.push({
			href: entry.href,
			rel: entry.rel ?? '',
			type: entry.type ?? '',
			size: parseSizes(entry.sizes, entry.type ?? ''),
			source: 'dom',
			order: idx,
		});
	});
	return candidates;
}

export function tabFaviconCandidate(
	favIconUrl: string | undefined,
	order: number,
): IconCandidate | null {
	if (!favIconUrl) return null;
	return {
		href: favIconUrl,
		rel: 'icon',
		type: '',
		size: 0,
		source: 'tab',
		order,
	};
}

export function originFallbackCandidate(
	pageUrl: string | undefined,
	order: number,
): IconCandidate | null {
	if (!pageUrl) return null;
	try {
		const fallback = new URL('/favicon.ico', pageUrl).href;
		return {
			href: fallback,
			rel: 'icon',
			type: 'image/x-icon',
			size: 0,
			source: 'origin',
			order,
		};
	} catch {
		return null;
	}
}

function dataUrlToBase64(url: string): string {
	const match = url.match(/^data:image\/[^;]+;base64,(.+)$/);
	return match ? match[1] : '';
}

export async function fetchIconAsBase64(href: string): Promise<string> {
	if (href.startsWith('data:')) {
		return dataUrlToBase64(href);
	}

	const response = await fetch(href, {
		credentials: 'omit',
		cache: 'force-cache',
		headers: { Accept: 'image/*,*/*;q=0.8' },
		redirect: 'follow',
	});

	if (!response.ok) {
		throw new Error(`Favicon fetch failed: ${response.status} ${response.statusText}`);
	}

	const contentType = (response.headers.get('content-type') || '').toLowerCase();
	if (
		contentType &&
		!contentType.startsWith('image/') &&
		contentType !== 'application/octet-stream'
	) {
		throw new Error(`Favicon response had unexpected content-type: ${contentType}`);
	}

	const blob = await response.blob();
	if (blob.size === 0) {
		throw new Error('Favicon response was empty');
	}

	return await new Promise<string>((resolve, reject) => {
		const reader = new FileReader();
		reader.onloadend = () => {
			const dataUrl = reader.result;
			if (typeof dataUrl !== 'string') {
				reject(new Error('FileReader returned non-string'));
				return;
			}
			const base64 = dataUrl.split(',')[1] || '';
			if (!base64) {
				reject(new Error('FileReader produced empty base64'));
				return;
			}
			resolve(base64);
		};
		reader.onerror = () => reject(reader.error ?? new Error('FileReader failed'));
		reader.readAsDataURL(blob);
	});
}

/**
 * Resolves the best favicon by trying each candidate in ranked order. Returns
 * "" if every candidate fails. Pure orchestrator — no browser or DOM API
 * required, callers supply the candidates.
 */
export async function resolveBestCandidate(candidates: IconCandidate[]): Promise<string> {
	const ranked = rankCandidates(candidates);
	const seen = new Set<string>();

	for (const candidate of ranked) {
		if (seen.has(candidate.href)) continue;
		seen.add(candidate.href);

		try {
			const base64 = await fetchIconAsBase64(candidate.href);
			if (base64) return base64;
		} catch (error) {
			console.warn(
				`Favicon candidate failed (${candidate.source}): ${candidate.href}`,
				error,
			);
		}
	}

	return '';
}
