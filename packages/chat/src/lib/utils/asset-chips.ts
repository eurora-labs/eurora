// Asset chips are stored on a `HumanMessage`'s `additional_kwargs` map under
// the `asset_chips` key. The wire type carries the kwargs as `unknown`; this
// helper does the small amount of runtime narrowing needed to surface chips
// to the chat UI.

import type { Message, AssetChip } from '$lib/models/messages/index.js';

interface KwargsWithAssetChips {
	asset_chips?: unknown;
}

function isObject(v: unknown): v is Record<string, unknown> {
	return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function asString(v: unknown): string | null {
	return typeof v === 'string' ? v : null;
}

export function getAssetChipsFromMessage(message: Message): AssetChip[] {
	if (message.type !== 'human') return [];
	const kwargs = message.additional_kwargs as KwargsWithAssetChips | undefined;
	if (!kwargs) return [];
	const raw = kwargs.asset_chips;
	if (!Array.isArray(raw)) return [];
	const chips: AssetChip[] = [];
	for (const entry of raw) {
		if (!isObject(entry)) continue;
		const id = asString(entry.id);
		const name = asString(entry.name);
		if (id === null || name === null) continue;
		chips.push({
			id,
			name,
			icon: asString(entry.icon),
			domain: asString(entry.domain),
		});
	}
	return chips;
}

export function getReasoningFromMessage(message: Message | undefined | null): string {
	if (!message || message.type === 'remove') return '';
	const fromBlocks = message.content
		.map((b) => (b.type === 'reasoning' ? (b.reasoning ?? '') : ''))
		.join('');
	if (fromBlocks) return fromBlocks;
	const kwargs = message.additional_kwargs as { reasoning_content?: unknown } | undefined;
	const reasoning = asString(kwargs?.reasoning_content);
	return reasoning ?? '';
}
