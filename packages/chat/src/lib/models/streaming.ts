import type { AIMessageChunk, ChatServerMessage } from '@eurora/shared/bindings/thread';

/// Re-exports of the agent-chain streaming primitives the chat package
/// surfaces. The wire envelope (`ChatServerMessage`) is consumed directly by
/// `ChatService` — there is no client-side reshaping layer.
export type AiMessageChunk = AIMessageChunk;
export type ToolCallChunk = AIMessageChunk['tool_call_chunks'][number];
export type { ChatServerMessage };
