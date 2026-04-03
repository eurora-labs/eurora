import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { InvalidToolCall, ToolCall, UsageMetadata } from '$lib/models/messages/ai-message.js';

export interface ToolCallChunk {
	name: string | null;
	args: string | null;
	id: string | null;
	index: number | null;
	chunkType: string | null;
}

export type ChunkPosition = 'last';

export interface AiMessageChunk {
	content: ContentBlock[];
	id: string | null;
	name: string | null;
	toolCalls: ToolCall[];
	invalidToolCalls: InvalidToolCall[];
	toolCallChunks: ToolCallChunk[];
	usageMetadata: UsageMetadata | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
	chunkPosition: ChunkPosition | null;
}
