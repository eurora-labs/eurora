import type {
	BaseMessageWithSibling,
	Block,
	ChatStreamResponse,
	Message,
	ProtoAiMessageChunk as RawAiMessageChunk,
	ProtoAnnotation,
	ProtoBlockIndex,
	ProtoContentBlock,
} from '$lib/bindings/bindings.js';
import type {
	Annotation,
	BlockIndex,
	ContentBlock,
} from '@eurora/chat/models/content-blocks/index';
import type { MessageNode, Message as DomainMessage } from '@eurora/chat/models/messages/index';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';

export function toMessageNodes(raw: BaseMessageWithSibling[]): MessageNode[] {
	return raw.map(toMessageNode);
}

function toMessageNode(raw: BaseMessageWithSibling): MessageNode {
	if (!raw.message?.message) {
		throw new Error(`MessageNode missing message (parent_id: ${raw.parent_id})`);
	}
	return {
		parentId: raw.parent_id,
		message: toMessage(raw.message.message),
		children: raw.children.map(toMessageNode),
		siblingIndex: raw.sibling_index,
		depth: raw.depth,
	};
}

function toMessage(raw: Message): DomainMessage {
	if ('Human' in raw) {
		return {
			type: 'human',
			content: raw.Human.content.map(toContentBlock),
			id: raw.Human.id ?? '',
			name: raw.Human.name,
			additionalKwargs: raw.Human.additional_kwargs,
			responseMetadata: raw.Human.response_metadata,
		};
	}
	if ('System' in raw) {
		return {
			type: 'system',
			content: raw.System.content.map(toContentBlock),
			id: raw.System.id ?? '',
			name: raw.System.name,
			additionalKwargs: raw.System.additional_kwargs,
			responseMetadata: raw.System.response_metadata,
		};
	}
	if ('Ai' in raw) {
		return {
			type: 'ai',
			content: raw.Ai.content.map(toContentBlock),
			id: raw.Ai.id ?? '',
			name: raw.Ai.name,
			toolCalls: raw.Ai.tool_calls.map((tc) => ({
				id: tc.id,
				name: tc.name,
				args: tc.args,
				callType: tc.call_type,
			})),
			invalidToolCalls: raw.Ai.invalid_tool_calls.map((tc) => ({
				id: tc.id,
				name: tc.name,
				args: tc.args,
				error: tc.error,
				callType: tc.call_type,
			})),
			usageMetadata: raw.Ai.usage_metadata
				? {
						inputTokens: raw.Ai.usage_metadata.input_tokens,
						outputTokens: raw.Ai.usage_metadata.output_tokens,
						totalTokens: raw.Ai.usage_metadata.total_tokens,
						inputTokenDetails: raw.Ai.usage_metadata.input_token_details
							? {
									audio: raw.Ai.usage_metadata.input_token_details.audio,
									cacheCreation:
										raw.Ai.usage_metadata.input_token_details.cache_creation,
									cacheRead: raw.Ai.usage_metadata.input_token_details.cache_read,
									extra: raw.Ai.usage_metadata.input_token_details.extra,
								}
							: null,
						outputTokenDetails: raw.Ai.usage_metadata.output_token_details
							? {
									audio: raw.Ai.usage_metadata.output_token_details.audio,
									reasoning: raw.Ai.usage_metadata.output_token_details.reasoning,
									extra: raw.Ai.usage_metadata.output_token_details.extra,
								}
							: null,
					}
				: null,
			additionalKwargs: raw.Ai.additional_kwargs,
			responseMetadata: raw.Ai.response_metadata,
		};
	}
	if ('Tool' in raw) {
		return {
			type: 'tool',
			content: raw.Tool.content.map(toContentBlock),
			toolCallId: raw.Tool.tool_call_id,
			id: raw.Tool.id ?? '',
			name: raw.Tool.name,
			status: raw.Tool.status,
			artifact: raw.Tool.artifact,
			additionalKwargs: raw.Tool.additional_kwargs,
			responseMetadata: raw.Tool.response_metadata,
		};
	}
	if ('Chat' in raw) {
		return {
			type: 'chat',
			content: raw.Chat.content.map(toContentBlock),
			role: raw.Chat.role,
			id: raw.Chat.id ?? '',
			name: raw.Chat.name,
			additionalKwargs: raw.Chat.additional_kwargs,
			responseMetadata: raw.Chat.response_metadata,
		};
	}
	return {
		type: 'remove',
		id: raw.Remove.id,
		name: raw.Remove.name,
		additionalKwargs: raw.Remove.additional_kwargs,
		responseMetadata: raw.Remove.response_metadata,
	};
}

function toContentBlock(raw: ProtoContentBlock): ContentBlock {
	if (!raw.block) {
		return { type: 'text', id: null, text: '', annotations: [], index: null, extras: null };
	}
	return toBlock(raw.block);
}

function toBlock(raw: Block): ContentBlock {
	if ('Text' in raw) {
		return {
			type: 'text',
			id: raw.Text.id,
			text: raw.Text.text,
			annotations: raw.Text.annotations.map(toAnnotation),
			index: toBlockIndex(raw.Text.index),
			extras: raw.Text.extras,
		};
	}
	if ('Reasoning' in raw) {
		return {
			type: 'reasoning',
			id: raw.Reasoning.id,
			reasoning: raw.Reasoning.reasoning,
			index: toBlockIndex(raw.Reasoning.index),
			extras: raw.Reasoning.extras,
		};
	}
	if ('Image' in raw) {
		return {
			type: 'image',
			id: raw.Image.id,
			fileId: raw.Image.file_id,
			mimeType: raw.Image.mime_type,
			index: toBlockIndex(raw.Image.index),
			url: raw.Image.url,
			base64: raw.Image.base64,
			extras: raw.Image.extras,
		};
	}
	if ('Video' in raw) {
		return {
			type: 'video',
			id: raw.Video.id,
			fileId: raw.Video.file_id,
			mimeType: raw.Video.mime_type,
			index: toBlockIndex(raw.Video.index),
			url: raw.Video.url,
			base64: raw.Video.base64,
			extras: raw.Video.extras,
		};
	}
	if ('Audio' in raw) {
		return {
			type: 'audio',
			id: raw.Audio.id,
			fileId: raw.Audio.file_id,
			mimeType: raw.Audio.mime_type,
			index: toBlockIndex(raw.Audio.index),
			url: raw.Audio.url,
			base64: raw.Audio.base64,
			extras: raw.Audio.extras,
		};
	}
	if ('PlainText' in raw) {
		return {
			type: 'plainText',
			id: raw.PlainText.id,
			fileId: raw.PlainText.file_id,
			mimeType: raw.PlainText.mime_type,
			index: toBlockIndex(raw.PlainText.index),
			url: raw.PlainText.url,
			base64: raw.PlainText.base64,
			text: raw.PlainText.text,
			title: raw.PlainText.title,
			context: raw.PlainText.context,
			extras: raw.PlainText.extras,
		};
	}
	if ('File' in raw) {
		return {
			type: 'file',
			id: raw.File.id,
			fileId: raw.File.file_id,
			mimeType: raw.File.mime_type,
			index: toBlockIndex(raw.File.index),
			url: raw.File.url,
			base64: raw.File.base64,
			extras: raw.File.extras,
		};
	}
	if ('NonStandard' in raw) {
		return {
			type: 'nonStandard',
			id: raw.NonStandard.id,
			value: raw.NonStandard.value,
			index: toBlockIndex(raw.NonStandard.index),
		};
	}
	if ('ToolCall' in raw) {
		return {
			type: 'toolCall',
			id: raw.ToolCall.id,
			name: raw.ToolCall.name,
			args: raw.ToolCall.args,
			index: toBlockIndex(raw.ToolCall.index),
			extras: raw.ToolCall.extras,
		};
	}
	if ('ToolCallChunk' in raw) {
		return {
			type: 'toolCallChunk',
			id: raw.ToolCallChunk.id,
			name: raw.ToolCallChunk.name,
			args: raw.ToolCallChunk.args,
			index: toBlockIndex(raw.ToolCallChunk.index),
			extras: raw.ToolCallChunk.extras,
		};
	}
	if ('InvalidToolCall' in raw) {
		return {
			type: 'invalidToolCall',
			id: raw.InvalidToolCall.id,
			name: raw.InvalidToolCall.name,
			args: raw.InvalidToolCall.args,
			error: raw.InvalidToolCall.error,
			index: toBlockIndex(raw.InvalidToolCall.index),
			extras: raw.InvalidToolCall.extras,
		};
	}
	if ('ServerToolCall' in raw) {
		return {
			type: 'serverToolCall',
			id: raw.ServerToolCall.id,
			name: raw.ServerToolCall.name,
			args: raw.ServerToolCall.args,
			index: toBlockIndex(raw.ServerToolCall.index),
			extras: raw.ServerToolCall.extras,
		};
	}
	if ('ServerToolCallChunk' in raw) {
		return {
			type: 'serverToolCallChunk',
			id: raw.ServerToolCallChunk.id,
			name: raw.ServerToolCallChunk.name,
			args: raw.ServerToolCallChunk.args,
			index: toBlockIndex(raw.ServerToolCallChunk.index),
			extras: raw.ServerToolCallChunk.extras,
		};
	}
	return {
		type: 'serverToolResult',
		id: raw.ServerToolResult.id,
		toolCallId: raw.ServerToolResult.tool_call_id,
		status: raw.ServerToolResult.status,
		output: raw.ServerToolResult.output,
		index: toBlockIndex(raw.ServerToolResult.index),
		extras: raw.ServerToolResult.extras,
	};
}

function toBlockIndex(raw: ProtoBlockIndex | null): BlockIndex | null {
	if (!raw?.index) return null;
	if ('IntIndex' in raw.index) return { type: 'int', value: raw.index.IntIndex };
	return { type: 'str', value: raw.index.StrIndex };
}

function toAnnotation(raw: ProtoAnnotation): Annotation {
	if (!raw.annotation) {
		return { type: 'nonStandard', value: { id: null, value: '' } };
	}
	if ('Citation' in raw.annotation) {
		return {
			type: 'citation',
			value: {
				id: raw.annotation.Citation.id,
				url: raw.annotation.Citation.url,
				title: raw.annotation.Citation.title,
				startIndex: raw.annotation.Citation.start_index,
				endIndex: raw.annotation.Citation.end_index,
				citedText: raw.annotation.Citation.cited_text,
				extras: raw.annotation.Citation.extras,
			},
		};
	}
	return {
		type: 'nonStandard',
		value: {
			id: raw.annotation.NonStandard.id,
			value: raw.annotation.NonStandard.value,
		},
	};
}

export function toChatStreamEvent(raw: ChatStreamResponse): ChatStreamEvent {
	if (!raw.payload) {
		throw new Error('ChatStreamResponse missing payload');
	}
	if ('FinalMessage' in raw.payload) {
		return {
			type: 'final',
			messages: toMessageNodes(raw.payload.FinalMessage.messages),
		};
	}
	return {
		type: 'chunk',
		chunk: toAiMessageChunk(raw.payload.Chunk),
	};
}

function toAiMessageChunk(raw: RawAiMessageChunk): AiMessageChunk {
	return {
		content: raw.content.map(toContentBlock),
		id: raw.id,
		name: raw.name,
		toolCalls: raw.tool_calls.map((tc) => ({
			id: tc.id,
			name: tc.name,
			args: tc.args,
			callType: tc.call_type,
		})),
		invalidToolCalls: raw.invalid_tool_calls.map((tc) => ({
			id: tc.id,
			name: tc.name,
			args: tc.args,
			error: tc.error,
			callType: tc.call_type,
		})),
		toolCallChunks: raw.tool_call_chunks.map((tc) => ({
			name: tc.name,
			args: tc.args,
			id: tc.id,
			index: tc.index,
			chunkType: tc.chunk_type,
		})),
		usageMetadata: raw.usage_metadata
			? {
					inputTokens: raw.usage_metadata.input_tokens,
					outputTokens: raw.usage_metadata.output_tokens,
					totalTokens: raw.usage_metadata.total_tokens,
					inputTokenDetails: raw.usage_metadata.input_token_details
						? {
								audio: raw.usage_metadata.input_token_details.audio,
								cacheCreation:
									raw.usage_metadata.input_token_details.cache_creation,
								cacheRead: raw.usage_metadata.input_token_details.cache_read,
								extra: raw.usage_metadata.input_token_details.extra,
							}
						: null,
					outputTokenDetails: raw.usage_metadata.output_token_details
						? {
								audio: raw.usage_metadata.output_token_details.audio,
								reasoning: raw.usage_metadata.output_token_details.reasoning,
								extra: raw.usage_metadata.output_token_details.extra,
							}
						: null,
				}
			: null,
		additionalKwargs: raw.additional_kwargs,
		responseMetadata: raw.response_metadata,
	};
}
