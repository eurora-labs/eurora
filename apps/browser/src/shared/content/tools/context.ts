import type { ContextResponse } from './types';

/// Build a `ContextResponse` carrying a single plain-text block. The
/// 99% case for per-site context summaries — sites that need richer
/// payloads (images, structured metadata) construct `ContextResponse`
/// directly.
export function textContext(message: string): ContextResponse {
	if (!message) {
		return { blocks: [] };
	}
	return {
		blocks: [
			{
				type: 'text',
				text: message,
			},
		],
	};
}
