export type {
	AiMessage,
	InvalidToolCall,
	InputTokenDetails,
	OutputTokenDetails,
	ToolCall,
	UsageMetadata,
} from './ai-message.js';
export type { ChatMessage } from './chat-message.js';
export type { HumanMessage } from './human-message.js';
export type { RemoveMessage } from './remove-message.js';
export type { SystemMessage } from './system-message.js';
export type { ToolMessage } from './tool-message.js';

import type { AiMessage } from '$lib/models/messages/ai-message.js';
import type { ChatMessage } from '$lib/models/messages/chat-message.js';
import type { HumanMessage } from '$lib/models/messages/human-message.js';
import type { RemoveMessage } from '$lib/models/messages/remove-message.js';
import type { SystemMessage } from '$lib/models/messages/system-message.js';
import type { ToolMessage } from '$lib/models/messages/tool-message.js';

export type Message =
	| HumanMessage
	| SystemMessage
	| AiMessage
	| ToolMessage
	| ChatMessage
	| RemoveMessage;

export interface MessageNode {
	parentId: string;
	message: Message;
	children: MessageNode[];
	siblingIndex: number;
	depth: number;
}
