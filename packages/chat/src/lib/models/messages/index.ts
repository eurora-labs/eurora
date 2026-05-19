// Wire types from `agent-chain-core` / `thread-core` are the canonical chat
// domain types. The chat package re-exports a curated view: the union of
// message variants is `Message`, the tree node is `MessageNode`, and
// narrow projections of both live in `./nodes.js`. The binding-level
// discriminator-less variants (`AIMessage`, `HumanMessage`, …) stay
// reachable through `@eurora/shared/bindings/thread` for the rare consumer
// that needs them — most code wants the discriminated versions exported
// from `./nodes.js` (`AiMessage`, `HumanMessage`, …) so a `switch`
// narrows correctly.

export type {
	AnyMessage as Message,
	AIMessageChunk as AiMessageChunk,
	MessageNode,
	ToolCall,
	ToolCallChunk,
	InvalidToolCall,
	UsageMetadata,
	InputTokenDetails,
	OutputTokenDetails,
} from '@eurora/shared/bindings/thread';

export type {
	AiMessage,
	HumanMessage,
	ToolMessage,
	SystemMessage,
	ChatMessage,
	RemoveMessage,
	AiNode,
	HumanNode,
	ToolNode,
	SystemNode,
} from './nodes.js';
export { isAiNode, isHumanNode, isToolNode, isSystemNode } from './nodes.js';

export type { AssetChip } from './asset-chip.js';

export {
	readAssetChips,
	writeAssetChips,
	readReasoningContent,
	appendReasoningContent,
	readChunkReasoningDelta,
} from './kwargs.js';

export type { PlaceholderId, LocalMessageId, LocalThreadId } from './ids.js';
export {
	newPlaceholderId,
	newLocalMessageId,
	newLocalThreadId,
	isPlaceholderId,
	isLocalMessageId,
	isLocalThreadId,
	isServerMessageId,
} from './ids.js';

export {
	createAiPlaceholderNode,
	createHumanPlaceholderNode,
	createLocalAiNode,
	createLocalHumanNode,
	createStubThread,
} from './factory.js';
export type { AiPlaceholderNode, HumanPlaceholderNode } from './factory.js';

export { AiStreamSink } from './stream-sink.js';
