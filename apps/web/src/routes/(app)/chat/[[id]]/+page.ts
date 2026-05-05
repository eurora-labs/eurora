import type { PageLoad } from './$types';

export function load({ params }: Parameters<PageLoad>[0]) {
	return {
		threadId: params.id ?? null,
	};
}
