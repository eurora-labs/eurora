import type {
	AIMessageChunk,
	ChatServerMessage,
	MessageNode,
} from '@eurora/shared/bindings/thread';

export type AiMessageChunk = AIMessageChunk;
export type ToolCallChunk = AIMessageChunk['tool_call_chunks'][number];

// Re-shape `ChatServerMessage` for consumers — the converter used to fold the
// `confirmed_human_message` and `error` variants into a single discriminated
// union. The error variant is now thrown as an exception inside the stream
// pipeline; consumers only see successful frames here.
export interface StreamConfirmedHumanMessage {
	type: 'confirmed_human';
	message: MessageNode;
}

export interface StreamChunk {
	type: 'chunk';
	chunk: AIMessageChunk;
}

export interface StreamFinalMessage {
	type: 'final';
	messages: MessageNode[];
}

export type ChatStreamEvent = StreamConfirmedHumanMessage | StreamChunk | StreamFinalMessage;

export function fromChatServerMessage(frame: ChatServerMessage): ChatStreamEvent {
	switch (frame.type) {
		case 'confirmed_human_message':
			return { type: 'confirmed_human', message: frame.message };
		case 'chunk':
			return { type: 'chunk', chunk: frame.chunk };
		case 'final':
			return { type: 'final', messages: frame.messages };
		case 'error':
			throw new Error(`${frame.kind}: ${frame.message}`);
	}
}
