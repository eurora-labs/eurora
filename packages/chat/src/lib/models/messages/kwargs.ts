// Typed accessors for the well-known keys that live on a message's
// `additional_kwargs`. Specta types the field as
// `{ [key in string]: unknown }`, so we narrow once here rather than at
// every callsite.
//
// Two well-known keys today:
//
// - `asset_chips` on a `HumanMessage` — the persisted display state for
//   the chips the user attached to their turn.
// - `reasoning_content` on an `AiMessage` (or chunk) — accumulated
//   reasoning emitted by providers like DeepSeek / Ollama / XAI that
//   surface chain-of-thought outside the content blocks.
//
// Readers accept the broad `Message` union so callers can pass a node's
// `.message` directly; mutators require the discriminator-narrowed types
// because writing to the wrong variant is a real bug.

import type { AssetChip } from '$lib/models/messages/asset-chip.js';
import type { Message } from '$lib/models/messages/index.js';
import type { AiMessage, HumanMessage } from '$lib/models/messages/nodes.js';

interface AdditionalKwargsLike {
	additional_kwargs?: { [key in string]: unknown };
}

function isObject(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function asString(value: unknown): string | null {
	return typeof value === 'string' ? value : null;
}

/**
 * Read the asset chips persisted on a human message. Returns an empty
 * array for non-human messages, missing kwargs, or entries that fail
 * the minimal shape (`id` + `name` are mandatory; `icon` / `domain`
 * default to null).
 */
export function readAssetChips(message: Message | null | undefined): AssetChip[] {
	if (!message || message.type !== 'human') return [];
	const raw = message.additional_kwargs?.asset_chips;
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

/**
 * Write the asset chips for a human message. Drops the key entirely if
 * the list is empty so the wire shape stays compact.
 */
export function writeAssetChips(message: HumanMessage, chips: AssetChip[]): void {
	const kwargs: Record<string, unknown> = isObject(message.additional_kwargs)
		? { ...message.additional_kwargs }
		: {};
	if (chips.length === 0) {
		delete kwargs.asset_chips;
	} else {
		kwargs.asset_chips = chips;
	}
	message.additional_kwargs = kwargs;
}

/**
 * Read the accumulated reasoning string from a message. Surveys both
 * content-block reasoning (the standard agent-chain placement) and the
 * `reasoning_content` kwarg (the additional_kwargs side-channel used by
 * some providers). Content blocks win when both are populated. Returns
 * the empty string for non-AI messages.
 */
export function readReasoningContent(message: Message | null | undefined): string {
	if (!message || message.type === 'remove') return '';
	const fromBlocks = message.content
		.map((block) => (block.type === 'reasoning' ? (block.reasoning ?? '') : ''))
		.join('');
	if (fromBlocks.length > 0) return fromBlocks;
	return asString(message.additional_kwargs?.reasoning_content) ?? '';
}

/**
 * Append a reasoning delta to the `reasoning_content` kwarg of an AI
 * message. No-op for empty deltas.
 */
export function appendReasoningContent(message: AiMessage, delta: string): void {
	if (delta.length === 0) return;
	const kwargs: Record<string, unknown> = isObject(message.additional_kwargs)
		? { ...message.additional_kwargs }
		: {};
	const previous = asString(kwargs.reasoning_content) ?? '';
	kwargs.reasoning_content = previous + delta;
	message.additional_kwargs = kwargs;
}

/**
 * Pull the `reasoning_content` delta off a chunk's `additional_kwargs`.
 * Returns the empty string when absent / non-string so callers can
 * concat unconditionally.
 */
export function readChunkReasoningDelta(chunk: AdditionalKwargsLike): string {
	return asString(chunk.additional_kwargs?.reasoning_content) ?? '';
}
