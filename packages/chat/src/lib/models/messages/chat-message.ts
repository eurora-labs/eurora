import type { ContentBlock } from '$lib/models/content-blocks/index.js';

export interface ChatMessage {
	type: 'chat';
	content: ContentBlock[];
	role: string;
	id: string;
	name: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
