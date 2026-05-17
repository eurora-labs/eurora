import type { MessageNode } from '$lib/models/messages/index.js';

export function getTextContent(node: MessageNode): string {
	const msg = node.message;
	if (msg.type === 'remove') return '';
	return msg.content.map((b) => (b.type === 'text' ? b.text : '')).join('');
}

/**
 * Returns the message's id, asserting that it is present.
 *
 * The wire type marks `id` optional because the Rust struct uses
 * `Option<String>`, but every message that reaches the UI carries an id —
 * persisted messages get one from the backend, streaming placeholders mint
 * one with `crypto.randomUUID()`. If this throws, something upstream is
 * passing a half-initialized `MessageNode` and the bug should be loud
 * rather than producing colliding `#each` keys.
 */
export function messageId(node: MessageNode): string {
	const id = node.message.id;
	if (typeof id === 'string' && id.length > 0) return id;
	throw new Error('MessageNode is missing an id');
}
