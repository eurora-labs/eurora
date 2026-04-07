import type { ContentBlock } from '$lib/models/content-blocks/index.js';

export interface ToolCall {
	id: string | null;
	name: string;
	args: string;
	callType: string | null;
}

export interface InvalidToolCall {
	name: string | null;
	args: string | null;
	id: string | null;
	error: string | null;
	callType: string | null;
}

export interface InputTokenDetails {
	audio: bigint | null;
	cacheCreation: bigint | null;
	cacheRead: bigint | null;
	extra: string | null;
}

export interface OutputTokenDetails {
	audio: bigint | null;
	reasoning: bigint | null;
	extra: string | null;
}

export interface UsageMetadata {
	inputTokens: bigint;
	outputTokens: bigint;
	totalTokens: bigint;
	inputTokenDetails: InputTokenDetails | null;
	outputTokenDetails: OutputTokenDetails | null;
}

export interface AiMessage {
	type: 'ai';
	content: ContentBlock[];
	id: string;
	name: string | null;
	toolCalls: ToolCall[];
	invalidToolCalls: InvalidToolCall[];
	usageMetadata: UsageMetadata | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
