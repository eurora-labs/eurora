import type { MessageNode } from '$lib/models/messages/index.js';

export type ChatStreamEvent =
	| { type: 'chunk'; content: string }
	| { type: 'reasoning'; content: string }
	| { type: 'done'; messages: MessageNode[] };
