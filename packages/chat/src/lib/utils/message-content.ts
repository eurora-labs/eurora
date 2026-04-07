import type { MessageNode } from '$lib/models/messages/index.js';

export function getTextContent(node: MessageNode): string {
	const msg = node.message;
	if (msg.type === 'remove') return '';
	return msg.content.map((b) => (b.type === 'text' ? b.text : '')).join('');
}
