// Wire types from `agent-chain-core` / `thread-core` are the canonical chat
// domain types. The chat package re-exports them so consumers don't need to
// know whether a type is generated or hand-rolled.

export type {
	AnyMessage as Message,
	AIMessage as AiMessage,
	AIMessageChunk as AiMessageChunk,
	HumanMessage,
	SystemMessage,
	ToolMessage,
	ChatMessage,
	RemoveMessage,
	ToolCall,
	ToolCallChunk,
	InvalidToolCall,
	UsageMetadata,
	InputTokenDetails,
	OutputTokenDetails,
	MessageNode,
} from '@eurora/shared/bindings/thread';
export type { AssetChip } from './asset-chip.js';
