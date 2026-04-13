import type { ContentBlock } from '$lib/models/content-blocks/index.js';

export interface SystemMessage {
	type: 'system';
	content: ContentBlock[];
	id: string;
	name: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
