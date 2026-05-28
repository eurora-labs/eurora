/**
 * Per-node truncation caps shared by every web tool. The numbers match
 * the Rust-side documentation on
 * `eurora_tools_web::types::QuerySelectorResult` and
 * `eurora_tools_web::types::ReadabilityArticle`.
 *
 * The text caps are byte-counted in UTF-16 (`string.length` units),
 * which over-counts surrogate pairs slightly but never under-counts —
 * a safe-by-construction bound for the bridge envelope.
 */
export const TEXT_NODE_CAP = 8 * 1024;
export const HTML_NODE_CAP = 32 * 1024;
export const READABILITY_BODY_CAP = 32 * 1024;

export const AX_TREE_DEFAULT_NODES = 500;
export const AX_TREE_HARD_CAP_NODES = 2000;
export const AX_TREE_DEFAULT_DEPTH = 12;

/**
 * Truncation outcome. `truncated` lets callers fold the per-call
 * `truncated` flag without re-comparing lengths.
 */
export interface ClampResult {
	value: string;
	truncated: boolean;
}

const ELLIPSIS = '…';

export function clampString(input: string, cap: number): ClampResult {
	if (input.length <= cap) {
		return { value: input, truncated: false };
	}
	// Cap-1 keeps the encoded result at or below `cap`; the ellipsis
	// surfaces truncation in any UI that displays the value raw.
	const head = input.slice(0, Math.max(0, cap - 1));
	return { value: `${head}${ELLIPSIS}`, truncated: true };
}

/**
 * Clamp the AX-tree-style numeric option to its policy band. `null`,
 * `undefined`, non-integers, and zero / negative values fall back to
 * `fallback`; values above `hardCap` are clamped to `hardCap`.
 */
export function clampNodeBudget(
	requested: number | null | undefined,
	fallback: number,
	hardCap: number,
): number {
	if (requested === null || requested === undefined) {
		return fallback;
	}
	if (!Number.isFinite(requested) || !Number.isInteger(requested) || requested <= 0) {
		return fallback;
	}
	return Math.min(requested, hardCap);
}
