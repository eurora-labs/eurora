import type { ContentBlock } from '$lib/models/content-blocks/index.js';

export interface ChatMessage {
	type: 'chat';
	content: ContentBlock[];
	role: string;
	id: string | null;
	name: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
