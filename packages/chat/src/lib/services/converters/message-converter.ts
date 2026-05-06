// Bridges the wire shapes coming back from the thread service over taurpc/HTTP
// (or directly via WebSocket on the web) into the strongly-typed domain
// models in `@eurora/chat`.
//
// The wire format is the JSON produced by agent-chain's serde impls plus the
// `MessageNode` / `ChatServerMessage` envelopes from the `thread-core` crate.
// Both inbound shapes are typed as `unknown` in the bindings file (see the
// `#[specta(type = Unknown)]` overrides in `thread-core::lib.rs`), so this
// module is the single place that does the runtime narrowing and validation.

import type { Annotation, BlockIndex, ContentBlock } from '$lib/models/content-blocks/index.js';
import type {
	AssetChip,
	MessageNode,
	Message as DomainMessage,
} from '$lib/models/messages/index.js';
import type { AiMessageChunk, ChatStreamEvent } from '$lib/models/streaming.js';

type Json = unknown;

function asObject(v: Json): Record<string, Json> | null {
	return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, Json>) : null;
}

function asString(v: Json): string | null {
	return typeof v === 'string' ? v : null;
}

function asNumber(v: Json): number | null {
	return typeof v === 'number' ? v : null;
}

function asArray(v: Json): Json[] {
	return Array.isArray(v) ? (v as Json[]) : [];
}

// agent-chain serializes `additional_kwargs`/`response_metadata` as plain
// JSON objects (HashMap<String, Value>); the domain models still expect the
// stringified form (legacy from the proto path), so we re-encode.
function stringifyKwargs(v: Json): string | null {
	if (v === undefined || v === null) return null;
	try {
		return JSON.stringify(v);
	} catch {
		return null;
	}
}

function parseAssetChipsFromKwargs(kwargs: Json): AssetChip[] {
	const obj = asObject(kwargs);
	if (!obj) return [];
	const raw = asArray(obj.asset_chips);
	const chips: AssetChip[] = [];
	for (const entry of raw) {
		const o = asObject(entry);
		if (!o) continue;
		const id = asString(o.id);
		const name = asString(o.name);
		if (id === null || name === null) continue;
		chips.push({
			id,
			name,
			icon: asString(o.icon),
			domain: asString(o.domain),
		});
	}
	return chips;
}

export function toMessageNodes(raw: Json[]): MessageNode[] {
	return raw.map(toMessageNode);
}

function toMessageNode(raw: Json): MessageNode {
	const obj = asObject(raw);
	if (!obj) {
		throw new Error('MessageNode is not an object');
	}
	const message = toMessage(obj.message);
	const children = asArray(obj.children).map(toMessageNode);
	return {
		parentId: asString(obj.parent_id),
		message,
		children,
		siblingIndex: asNumber(obj.sibling_index) ?? 0,
		depth: asNumber(obj.depth) ?? 0,
	};
}

function toMessage(raw: Json): DomainMessage {
	const obj = asObject(raw);
	if (!obj) {
		throw new Error('Message payload is not an object');
	}
	const type = asString(obj.type);
	switch (type) {
		case 'human':
			return {
				type: 'human',
				content: asArray(obj.content).map(toContentBlock),
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
				assetChips: parseAssetChipsFromKwargs(obj.additional_kwargs),
			};
		case 'system':
			return {
				type: 'system',
				content: asArray(obj.content).map(toContentBlock),
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
			};
		case 'ai':
		case 'AIMessageChunk':
			return {
				type: 'ai',
				content: asArray(obj.content).map(toContentBlock),
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				toolCalls: asArray(obj.tool_calls).map(toToolCall),
				invalidToolCalls: asArray(obj.invalid_tool_calls).map(toInvalidToolCall),
				usageMetadata: toUsageMetadata(obj.usage_metadata),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
			};
		case 'tool':
			return {
				type: 'tool',
				content: asArray(obj.content).map(toContentBlock),
				toolCallId: asString(obj.tool_call_id) ?? '',
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				status: asNumber(obj.status) ?? 0,
				artifact: stringifyKwargs(obj.artifact),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
			};
		case 'chat':
			return {
				type: 'chat',
				content: asArray(obj.content).map(toContentBlock),
				role: asString(obj.role) ?? '',
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
			};
		case 'remove':
			return {
				type: 'remove',
				id: asString(obj.id) ?? '',
				name: asString(obj.name),
				additionalKwargs: stringifyKwargs(obj.additional_kwargs),
				responseMetadata: stringifyKwargs(obj.response_metadata),
			};
		default:
			throw new Error(`Unknown message type: ${String(type)}`);
	}
}

function toContentBlock(raw: Json): ContentBlock {
	const obj = asObject(raw);
	if (!obj) {
		return { type: 'text', id: null, text: '', annotations: [], index: null, extras: null };
	}
	const type = asString(obj.type);
	switch (type) {
		case 'text':
			return {
				type: 'text',
				id: asString(obj.id),
				text: asString(obj.text) ?? '',
				annotations: asArray(obj.annotations).map(toAnnotation),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'reasoning':
			return {
				type: 'reasoning',
				id: asString(obj.id),
				reasoning: asString(obj.reasoning),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'image':
			return {
				type: 'image',
				id: asString(obj.id),
				fileId: asString(obj.file_id),
				mimeType: asString(obj.mime_type),
				index: toBlockIndex(obj.index),
				url: asString(obj.url),
				base64: asString(obj.base64),
				extras: stringifyKwargs(obj.extras),
			};
		case 'video':
			return {
				type: 'video',
				id: asString(obj.id),
				fileId: asString(obj.file_id),
				mimeType: asString(obj.mime_type),
				index: toBlockIndex(obj.index),
				url: asString(obj.url),
				base64: asString(obj.base64),
				extras: stringifyKwargs(obj.extras),
			};
		case 'audio':
			return {
				type: 'audio',
				id: asString(obj.id),
				fileId: asString(obj.file_id),
				mimeType: asString(obj.mime_type),
				index: toBlockIndex(obj.index),
				url: asString(obj.url),
				base64: asString(obj.base64),
				extras: stringifyKwargs(obj.extras),
			};
		case 'text-plain':
			return {
				type: 'plainText',
				id: asString(obj.id),
				fileId: asString(obj.file_id),
				mimeType: asString(obj.mime_type) ?? '',
				index: toBlockIndex(obj.index),
				url: asString(obj.url),
				base64: asString(obj.base64),
				text: asString(obj.text),
				title: asString(obj.title),
				context: asString(obj.context),
				extras: stringifyKwargs(obj.extras),
			};
		case 'file':
			return {
				type: 'file',
				id: asString(obj.id),
				fileId: asString(obj.file_id),
				mimeType: asString(obj.mime_type),
				index: toBlockIndex(obj.index),
				url: asString(obj.url),
				base64: asString(obj.base64),
				extras: stringifyKwargs(obj.extras),
			};
		case 'non_standard':
			return {
				type: 'nonStandard',
				id: asString(obj.id),
				value: stringifyKwargs(obj.value) ?? '',
				index: toBlockIndex(obj.index),
			};
		case 'tool_call':
			return {
				type: 'toolCall',
				id: asString(obj.id),
				name: asString(obj.name) ?? '',
				args: stringifyKwargs(obj.args) ?? '',
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'tool_call_chunk':
			return {
				type: 'toolCallChunk',
				id: asString(obj.id),
				name: asString(obj.name),
				args: asString(obj.args),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'invalid_tool_call':
			return {
				type: 'invalidToolCall',
				id: asString(obj.id),
				name: asString(obj.name),
				args: asString(obj.args),
				error: asString(obj.error),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'server_tool_call':
			return {
				type: 'serverToolCall',
				id: asString(obj.id) ?? '',
				name: asString(obj.name) ?? '',
				args: stringifyKwargs(obj.args) ?? '',
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'server_tool_call_chunk':
			return {
				type: 'serverToolCallChunk',
				id: asString(obj.id),
				name: asString(obj.name),
				args: asString(obj.args),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		case 'server_tool_result':
			return {
				type: 'serverToolResult',
				id: asString(obj.id),
				toolCallId: asString(obj.tool_call_id) ?? '',
				status: asNumber(obj.status) ?? 0,
				output: stringifyKwargs(obj.output),
				index: toBlockIndex(obj.index),
				extras: stringifyKwargs(obj.extras),
			};
		default:
			return {
				type: 'nonStandard',
				id: asString(obj.id),
				value: stringifyKwargs(obj) ?? '',
				index: toBlockIndex(obj.index),
			};
	}
}

function toBigInt(raw: Json): bigint | null {
	if (raw === null || raw === undefined) return null;
	if (typeof raw === 'bigint') return raw;
	if (typeof raw === 'number' && Number.isFinite(raw)) return BigInt(Math.trunc(raw));
	if (typeof raw === 'string') {
		try {
			return BigInt(raw);
		} catch {
			return null;
		}
	}
	return null;
}

function toBlockIndex(raw: Json): BlockIndex | null {
	if (raw === undefined || raw === null) return null;
	if (typeof raw === 'number') {
		const v = toBigInt(raw);
		return v !== null ? { type: 'int', value: v } : null;
	}
	if (typeof raw === 'string') return { type: 'str', value: raw };
	const obj = asObject(raw);
	if (!obj) return null;
	const intIdx = toBigInt(obj.int_index);
	if (intIdx !== null) return { type: 'int', value: intIdx };
	const strIdx = asString(obj.str_index);
	if (strIdx !== null) return { type: 'str', value: strIdx };
	return null;
}

function toAnnotation(raw: Json): Annotation {
	const obj = asObject(raw);
	if (!obj) return { type: 'nonStandard', value: { id: null, value: '' } };
	const type = asString(obj.type);
	if (type === 'citation') {
		return {
			type: 'citation',
			value: {
				id: asString(obj.id),
				url: asString(obj.url),
				title: asString(obj.title),
				startIndex: toBigInt(obj.start_index),
				endIndex: toBigInt(obj.end_index),
				citedText: asString(obj.cited_text),
				extras: stringifyKwargs(obj.extras),
			},
		};
	}
	return {
		type: 'nonStandard',
		value: {
			id: asString(obj.id),
			value: stringifyKwargs(obj.value) ?? '',
		},
	};
}

function toToolCall(raw: Json) {
	const obj = asObject(raw) ?? {};
	return {
		id: asString(obj.id),
		name: asString(obj.name) ?? '',
		args: stringifyKwargs(obj.args) ?? '',
		callType: asString(obj.type),
	};
}

function toInvalidToolCall(raw: Json) {
	const obj = asObject(raw) ?? {};
	return {
		id: asString(obj.id),
		name: asString(obj.name),
		args: asString(obj.args),
		error: asString(obj.error),
		callType: asString(obj.type),
	};
}

function toUsageMetadata(raw: Json) {
	const obj = asObject(raw);
	if (!obj) return null;
	const inputDetails = asObject(obj.input_token_details);
	const outputDetails = asObject(obj.output_token_details);
	return {
		inputTokens: toBigInt(obj.input_tokens) ?? 0n,
		outputTokens: toBigInt(obj.output_tokens) ?? 0n,
		totalTokens: toBigInt(obj.total_tokens) ?? 0n,
		inputTokenDetails: inputDetails
			? {
					audio: toBigInt(inputDetails.audio),
					cacheCreation: toBigInt(inputDetails.cache_creation),
					cacheRead: toBigInt(inputDetails.cache_read),
					extra: stringifyKwargs(inputDetails),
				}
			: null,
		outputTokenDetails: outputDetails
			? {
					audio: toBigInt(outputDetails.audio),
					reasoning: toBigInt(outputDetails.reasoning),
					extra: stringifyKwargs(outputDetails),
				}
			: null,
	};
}

export function toChatStreamEvent(raw: Json): ChatStreamEvent {
	const obj = asObject(raw);
	if (!obj) {
		throw new Error('ChatServerMessage is not an object');
	}
	const type = asString(obj.type);
	switch (type) {
		case 'confirmed_human_message':
			return {
				type: 'confirmed_human',
				message: toMessageNode(obj.message),
			};
		case 'chunk':
			return {
				type: 'chunk',
				chunk: toAiMessageChunk(obj.chunk),
			};
		case 'final':
			return {
				type: 'final',
				messages: toMessageNodes(asArray(obj.messages)),
			};
		case 'error': {
			const message = asString(obj.message) ?? 'Chat stream error';
			const kind = asString(obj.kind) ?? 'unknown';
			throw new Error(`${kind}: ${message}`);
		}
		default:
			throw new Error(`Unknown ChatServerMessage variant: ${String(type)}`);
	}
}

function toAiMessageChunk(raw: Json): AiMessageChunk {
	const obj = asObject(raw) ?? {};
	return {
		content: asArray(obj.content).map(toContentBlock),
		id: asString(obj.id),
		name: asString(obj.name),
		toolCalls: asArray(obj.tool_calls).map(toToolCall),
		invalidToolCalls: asArray(obj.invalid_tool_calls).map(toInvalidToolCall),
		toolCallChunks: asArray(obj.tool_call_chunks).map((c) => {
			const o = asObject(c) ?? {};
			return {
				name: asString(o.name),
				args: asString(o.args),
				id: asString(o.id),
				index: asNumber(o.index),
				chunkType: asString(o.type),
			};
		}),
		usageMetadata: toUsageMetadata(obj.usage_metadata),
		additionalKwargs: stringifyKwargs(obj.additional_kwargs),
		responseMetadata: stringifyKwargs(obj.response_metadata),
	};
}
