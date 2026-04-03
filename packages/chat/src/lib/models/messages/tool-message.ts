import type { ContentBlock } from '$lib/models/content-blocks/index.js';

export interface ToolMessage {
	type: 'tool';
	content: ContentBlock[];
	toolCallId: string;
	id: string | null;
	name: string | null;
	status: number;
	artifact: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
