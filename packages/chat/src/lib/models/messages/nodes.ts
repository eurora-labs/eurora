// Narrow projections of the chat tree-node and message types.
//
// `MessageNode` (the tree node) and `AnyMessage` (the message union) come
// from specta-generated bindings. The bindings are wire-faithful but
// permissive — a `MessageNode` has `message: AnyMessage` and callers narrow
// by discriminator at every touchpoint.
//
// This module pins the narrow shapes once so consumers can pass typed
// values around without re-narrowing. `AiMessage` is the
// discriminator-bearing AI variant of `AnyMessage` (not the same as
// `AIMessage` from bindings, which omits the `type` tag). `AiNode` is a
// `MessageNode` whose `.message` is fixed to that variant — and likewise
// for the other message kinds.

import type { AnyMessage, MessageNode } from '@eurora/shared/bindings/thread';

export type AiMessage = Extract<AnyMessage, { type: 'ai' }>;
export type HumanMessage = Extract<AnyMessage, { type: 'human' }>;
export type ToolMessage = Extract<AnyMessage, { type: 'tool' }>;
export type SystemMessage = Extract<AnyMessage, { type: 'system' }>;
export type ChatMessage = Extract<AnyMessage, { type: 'chat' }>;
export type RemoveMessage = Extract<AnyMessage, { type: 'remove' }>;

export type AiNode = MessageNode & { message: AiMessage };
export type HumanNode = MessageNode & { message: HumanMessage };
export type ToolNode = MessageNode & { message: ToolMessage };
export type SystemNode = MessageNode & { message: SystemMessage };

export function isAiNode(node: MessageNode): node is AiNode {
	return node.message.type === 'ai';
}

export function isHumanNode(node: MessageNode): node is HumanNode {
	return node.message.type === 'human';
}

export function isToolNode(node: MessageNode): node is ToolNode {
	return node.message.type === 'tool';
}

export function isSystemNode(node: MessageNode): node is SystemNode {
	return node.message.type === 'system';
}
