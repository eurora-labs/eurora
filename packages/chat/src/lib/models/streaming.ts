import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { InvalidToolCall, ToolCall, UsageMetadata } from '$lib/models/messages/ai-message.js';
import type { MessageNode } from '$lib/models/messages/index.js';

export interface ToolCallChunk {
	name: string | null;
	args: string | null;
	id: string | null;
	index: number | null;
	chunkType: string | null;
}

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
}

export interface StreamChunk {
	type: 'chunk';
	chunk: AiMessageChunk;
}

export interface StreamFinalMessage {
	type: 'final';
	messages: MessageNode[];
}

export interface StreamConfirmedHumanMessage {
	type: 'confirmed_human';
	message: MessageNode;
}

export type ChatStreamEvent = StreamChunk | StreamFinalMessage | StreamConfirmedHumanMessage;
