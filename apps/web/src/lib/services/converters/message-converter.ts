import type { Timestamp } from '@bufbuild/protobuf/wkt';
import type {
	Annotation,
	BlockIndex,
	ContentBlock,
} from '@eurora/chat/models/content-blocks/index';
import type {
	AssetChip,
	Message as DomainMessage,
	MessageNode,
} from '@eurora/chat/models/messages/index';
import type { AiMessageChunk, ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	BaseMessageWithSibling,
	ChatStreamResponse,
	ProtoAIMessageChunk,
	ProtoAnnotation,
	ProtoBaseMessage,
	ProtoBlockIndex,
	ProtoContentBlock,
} from '@eurora/shared/proto/agent_chain_pb.js';
import type { ProtoThread } from '@eurora/shared/proto/thread_service_pb.js';

export function toThread(raw: ProtoThread | undefined): Thread {
	if (!raw?.id) {
		throw new Error('Backend returned thread without id');
	}
	return {
		id: raw.id,
		title: raw.title,
		createdAt: timestampToIso(raw.createdAt),
		updatedAt: timestampToIso(raw.updatedAt),
	};
}

export function toMessageNodes(raw: BaseMessageWithSibling[]): MessageNode[] {
	return raw.map(toMessageNode);
}

function toMessageNode(raw: BaseMessageWithSibling): MessageNode {
	if (!raw.message) {
		throw new Error(`MessageNode missing message (parentId: ${raw.parentId})`);
	}
	return {
		parentId: raw.parentId || null,
		message: toMessage(raw.message),
		children: raw.children.map(toMessageNode),
		siblingIndex: raw.siblingIndex,
		depth: raw.depth,
	};
}

function toMessage(raw: ProtoBaseMessage): DomainMessage {
	switch (raw.message.case) {
		case 'human': {
			const m = raw.message.value;
			return {
				type: 'human',
				content: m.content.map(toContentBlock),
				id: m.id ?? '',
				name: m.name ?? null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
				assetChips: parseAssetChips(m.additionalKwargs),
			};
		}
		case 'system': {
			const m = raw.message.value;
			return {
				type: 'system',
				content: m.content.map(toContentBlock),
				id: m.id ?? '',
				name: m.name ?? null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
			};
		}
		case 'ai': {
			const m = raw.message.value;
			return {
				type: 'ai',
				content: m.content.map(toContentBlock),
				id: m.id ?? '',
				name: m.name ?? null,
				toolCalls: m.toolCalls.map((tc) => ({
					id: tc.id ?? null,
					name: tc.name,
					args: tc.args,
					callType: tc.callType ?? null,
				})),
				invalidToolCalls: m.invalidToolCalls.map((tc) => ({
					id: tc.id ?? null,
					name: tc.name ?? null,
					args: tc.args ?? null,
					error: tc.error ?? null,
					callType: tc.callType ?? null,
				})),
				usageMetadata: m.usageMetadata
					? {
							inputTokens: m.usageMetadata.inputTokens,
							outputTokens: m.usageMetadata.outputTokens,
							totalTokens: m.usageMetadata.totalTokens,
							inputTokenDetails: m.usageMetadata.inputTokenDetails
								? {
										audio: m.usageMetadata.inputTokenDetails.audio ?? null,
										cacheCreation:
											m.usageMetadata.inputTokenDetails.cacheCreation ?? null,
										cacheRead:
											m.usageMetadata.inputTokenDetails.cacheRead ?? null,
										extra: m.usageMetadata.inputTokenDetails.extra ?? null,
									}
								: null,
							outputTokenDetails: m.usageMetadata.outputTokenDetails
								? {
										audio: m.usageMetadata.outputTokenDetails.audio ?? null,
										reasoning:
											m.usageMetadata.outputTokenDetails.reasoning ?? null,
										extra: m.usageMetadata.outputTokenDetails.extra ?? null,
									}
								: null,
						}
					: null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
			};
		}
		case 'tool': {
			const m = raw.message.value;
			return {
				type: 'tool',
				content: m.content.map(toContentBlock),
				toolCallId: m.toolCallId,
				id: m.id ?? '',
				name: m.name ?? null,
				status: m.status,
				artifact: m.artifact ?? null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
			};
		}
		case 'chat': {
			const m = raw.message.value;
			return {
				type: 'chat',
				content: m.content.map(toContentBlock),
				role: m.role,
				id: m.id ?? '',
				name: m.name ?? null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
			};
		}
		case 'remove': {
			const m = raw.message.value;
			return {
				type: 'remove',
				id: m.id,
				name: m.name ?? null,
				additionalKwargs: m.additionalKwargs ?? null,
				responseMetadata: m.responseMetadata ?? null,
			};
		}
		default:
			throw new Error('ProtoBaseMessage missing variant');
	}
}

function toContentBlock(raw: ProtoContentBlock): ContentBlock {
	switch (raw.block.case) {
		case 'text': {
			const b = raw.block.value;
			return {
				type: 'text',
				id: b.id ?? null,
				text: b.text,
				annotations: b.annotations.map(toAnnotation),
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'reasoning': {
			const b = raw.block.value;
			return {
				type: 'reasoning',
				id: b.id ?? null,
				reasoning: b.reasoning ?? null,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'image': {
			const b = raw.block.value;
			return {
				type: 'image',
				id: b.id ?? null,
				fileId: b.fileId ?? null,
				mimeType: b.mimeType ?? null,
				index: toBlockIndex(b.index),
				url: b.url ?? null,
				base64: b.base64 ?? null,
				extras: b.extras ?? null,
			};
		}
		case 'video': {
			const b = raw.block.value;
			return {
				type: 'video',
				id: b.id ?? null,
				fileId: b.fileId ?? null,
				mimeType: b.mimeType ?? null,
				index: toBlockIndex(b.index),
				url: b.url ?? null,
				base64: b.base64 ?? null,
				extras: b.extras ?? null,
			};
		}
		case 'audio': {
			const b = raw.block.value;
			return {
				type: 'audio',
				id: b.id ?? null,
				fileId: b.fileId ?? null,
				mimeType: b.mimeType ?? null,
				index: toBlockIndex(b.index),
				url: b.url ?? null,
				base64: b.base64 ?? null,
				extras: b.extras ?? null,
			};
		}
		case 'plainText': {
			const b = raw.block.value;
			return {
				type: 'plainText',
				id: b.id ?? null,
				fileId: b.fileId ?? null,
				mimeType: b.mimeType,
				index: toBlockIndex(b.index),
				url: b.url ?? null,
				base64: b.base64 ?? null,
				text: b.text ?? null,
				title: b.title ?? null,
				context: b.context ?? null,
				extras: b.extras ?? null,
			};
		}
		case 'file': {
			const b = raw.block.value;
			return {
				type: 'file',
				id: b.id ?? null,
				fileId: b.fileId ?? null,
				mimeType: b.mimeType ?? null,
				index: toBlockIndex(b.index),
				url: b.url ?? null,
				base64: b.base64 ?? null,
				extras: b.extras ?? null,
			};
		}
		case 'nonStandard': {
			const b = raw.block.value;
			return {
				type: 'nonStandard',
				id: b.id ?? null,
				value: b.value,
				index: toBlockIndex(b.index),
			};
		}
		case 'toolCall': {
			const b = raw.block.value;
			return {
				type: 'toolCall',
				id: b.id ?? null,
				name: b.name,
				args: b.args,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'toolCallChunk': {
			const b = raw.block.value;
			return {
				type: 'toolCallChunk',
				id: b.id ?? null,
				name: b.name ?? null,
				args: b.args ?? null,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'invalidToolCall': {
			const b = raw.block.value;
			return {
				type: 'invalidToolCall',
				id: b.id ?? null,
				name: b.name ?? null,
				args: b.args ?? null,
				error: b.error ?? null,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'serverToolCall': {
			const b = raw.block.value;
			return {
				type: 'serverToolCall',
				id: b.id,
				name: b.name,
				args: b.args,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'serverToolCallChunk': {
			const b = raw.block.value;
			return {
				type: 'serverToolCallChunk',
				id: b.id ?? null,
				name: b.name ?? null,
				args: b.args ?? null,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		case 'serverToolResult': {
			const b = raw.block.value;
			return {
				type: 'serverToolResult',
				id: b.id ?? null,
				toolCallId: b.toolCallId,
				status: b.status,
				output: b.output ?? null,
				index: toBlockIndex(b.index),
				extras: b.extras ?? null,
			};
		}
		default:
			return { type: 'text', id: null, text: '', annotations: [], index: null, extras: null };
	}
}

function toBlockIndex(raw: ProtoBlockIndex | undefined): BlockIndex | null {
	if (!raw) return null;
	switch (raw.index.case) {
		case 'intIndex':
			return { type: 'int', value: raw.index.value };
		case 'strIndex':
			return { type: 'str', value: raw.index.value };
		default:
			return null;
	}
}

function toAnnotation(raw: ProtoAnnotation): Annotation {
	switch (raw.annotation.case) {
		case 'citation': {
			const a = raw.annotation.value;
			return {
				type: 'citation',
				value: {
					id: a.id ?? null,
					url: a.url ?? null,
					title: a.title ?? null,
					startIndex: a.startIndex ?? null,
					endIndex: a.endIndex ?? null,
					citedText: a.citedText ?? null,
					extras: a.extras ?? null,
				},
			};
		}
		case 'nonStandard': {
			const a = raw.annotation.value;
			return {
				type: 'nonStandard',
				value: { id: a.id ?? null, value: a.value },
			};
		}
		default:
			return { type: 'nonStandard', value: { id: null, value: '' } };
	}
}

export function toChatStreamEvent(raw: ChatStreamResponse): ChatStreamEvent {
	switch (raw.payload.case) {
		case 'chunk':
			return { type: 'chunk', chunk: toAiMessageChunk(raw.payload.value) };
		case 'finalMessage':
			return { type: 'final', messages: toMessageNodes(raw.payload.value.messages) };
		case 'confirmedHumanMessage':
			return { type: 'confirmed_human', message: toMessageNode(raw.payload.value) };
		default:
			throw new Error('ChatStreamResponse missing payload');
	}
}

function toAiMessageChunk(raw: ProtoAIMessageChunk): AiMessageChunk {
	return {
		content: raw.content.map(toContentBlock),
		id: raw.id ?? null,
		name: raw.name ?? null,
		toolCalls: raw.toolCalls.map((tc) => ({
			id: tc.id ?? null,
			name: tc.name,
			args: tc.args,
			callType: tc.callType ?? null,
		})),
		invalidToolCalls: raw.invalidToolCalls.map((tc) => ({
			id: tc.id ?? null,
			name: tc.name ?? null,
			args: tc.args ?? null,
			error: tc.error ?? null,
			callType: tc.callType ?? null,
		})),
		toolCallChunks: raw.toolCallChunks.map((tc) => ({
			name: tc.name ?? null,
			args: tc.args ?? null,
			id: tc.id ?? null,
			index: tc.index ?? null,
			chunkType: tc.chunkType ?? null,
		})),
		usageMetadata: raw.usageMetadata
			? {
					inputTokens: raw.usageMetadata.inputTokens,
					outputTokens: raw.usageMetadata.outputTokens,
					totalTokens: raw.usageMetadata.totalTokens,
					inputTokenDetails: raw.usageMetadata.inputTokenDetails
						? {
								audio: raw.usageMetadata.inputTokenDetails.audio ?? null,
								cacheCreation:
									raw.usageMetadata.inputTokenDetails.cacheCreation ?? null,
								cacheRead: raw.usageMetadata.inputTokenDetails.cacheRead ?? null,
								extra: raw.usageMetadata.inputTokenDetails.extra ?? null,
							}
						: null,
					outputTokenDetails: raw.usageMetadata.outputTokenDetails
						? {
								audio: raw.usageMetadata.outputTokenDetails.audio ?? null,
								reasoning: raw.usageMetadata.outputTokenDetails.reasoning ?? null,
								extra: raw.usageMetadata.outputTokenDetails.extra ?? null,
							}
						: null,
				}
			: null,
		additionalKwargs: raw.additionalKwargs ?? null,
		responseMetadata: raw.responseMetadata ?? null,
	};
}

function parseAssetChips(additionalKwargs: string | undefined): AssetChip[] {
	if (!additionalKwargs) return [];
	let parsed: unknown;
	try {
		parsed = JSON.parse(additionalKwargs);
	} catch {
		return [];
	}
	if (!parsed || typeof parsed !== 'object') return [];
	const raw = (parsed as Record<string, unknown>).asset_chips;
	if (!Array.isArray(raw)) return [];
	const chips: AssetChip[] = [];
	for (const entry of raw) {
		if (!entry || typeof entry !== 'object') continue;
		const obj = entry as Record<string, unknown>;
		if (typeof obj.id !== 'string' || typeof obj.name !== 'string') continue;
		chips.push({
			id: obj.id,
			name: obj.name,
			icon: typeof obj.icon === 'string' ? obj.icon : null,
			domain: typeof obj.domain === 'string' ? obj.domain : null,
		});
	}
	return chips;
}

function timestampToIso(ts: Timestamp | undefined): string | undefined {
	if (!ts) return undefined;
	const ms = Number(ts.seconds) * 1000 + Math.floor(ts.nanos / 1_000_000);
	return new Date(ms).toISOString();
}
