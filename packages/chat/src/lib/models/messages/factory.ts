// Factories for the chat-service's view-model nodes.
//
// All construction of `MessageNode` literals goes through here so the
// wire shape lives in one place. Returned types are *narrow* — a caller
// asking for a placeholder AI node receives an `AiNode`, not a
// `MessageNode` that must be re-narrowed before use. The IDs are minted
// inside the factories so the call site can't accidentally hand out a
// raw UUID where a `placeholder:`-prefixed one is expected.

import { newLocalMessageId, newLocalThreadId, newPlaceholderId } from '$lib/models/messages/ids.js';
import { writeAssetChips } from '$lib/models/messages/kwargs.js';
import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { AssetChip } from '$lib/models/messages/asset-chip.js';
import type { AiNode, HumanNode } from '$lib/models/messages/nodes.js';
import type { Thread } from '$lib/models/thread.model.js';

// The wire types declare `message.id` as `string | null | undefined`
// because the Rust struct uses `Option<String>`. Every node minted by
// these factories *does* set a fresh placeholder/local id, so the
// returned type asserts that — callers don't need to coalesce at every
// reference. `messageId()` from `utils/message-content.ts` enforces the
// same invariant for nodes coming back from the server.
type WithId<TNode extends AiNode | HumanNode> = TNode & {
	message: TNode['message'] & { id: string };
};

export type AiPlaceholderNode = WithId<AiNode>;
export type HumanPlaceholderNode = WithId<HumanNode>;

function textBlock(text: string): ContentBlock {
	return {
		type: 'text',
		id: null,
		text,
		annotations: null,
		index: null,
		extras: null,
	};
}

/**
 * Build a streaming-AI placeholder node. The id is a fresh
 * `placeholder:`-prefixed string; the caller is expected to swap the
 * whole node for the persisted one when the server emits `final`.
 */
export function createAiPlaceholderNode(parentId: string | null, text = ''): AiPlaceholderNode {
	return {
		parent_id: parentId,
		message: {
			type: 'ai',
			id: newPlaceholderId(),
			name: null,
			content: text.length > 0 ? [textBlock(text)] : [],
			tool_calls: [],
			invalid_tool_calls: [],
			usage_metadata: null,
			additional_kwargs: {},
			response_metadata: {},
		},
		children: [],
		sibling_index: 0,
		depth: 0,
	};
}

/**
 * Build a human-message placeholder node. The id is a fresh
 * `placeholder:`-prefixed string; the caller swaps it for the persisted
 * node when the server returns `confirmed_human_message`.
 */
export function createHumanPlaceholderNode(
	parentId: string | null,
	text: string,
	assetChips: AssetChip[] = [],
): HumanPlaceholderNode {
	const node: HumanPlaceholderNode = {
		parent_id: parentId,
		message: {
			type: 'human',
			id: newPlaceholderId(),
			name: null,
			content: text.length > 0 ? [textBlock(text)] : [],
			additional_kwargs: {},
			response_metadata: {},
		},
		children: [],
		sibling_index: 0,
		depth: 0,
	};
	if (assetChips.length > 0) {
		writeAssetChips(node.message, assetChips);
	}
	return node;
}

/**
 * Build a never-syncs AI node for the transient demo thread. The id is
 * a fresh `local:`-prefixed string.
 */
export function createLocalAiNode(parentId: string | null, text: string): AiPlaceholderNode {
	const node = createAiPlaceholderNode(parentId, text);
	node.message.id = newLocalMessageId();
	return node;
}

/**
 * Build a never-syncs human node for the transient demo thread. The id
 * is a fresh `local:`-prefixed string.
 */
export function createLocalHumanNode(
	parentId: string | null,
	text: string,
	assetChips: AssetChip[] = [],
): HumanPlaceholderNode {
	const node = createHumanPlaceholderNode(parentId, text, assetChips);
	node.message.id = newLocalMessageId();
	return node;
}

/**
 * Stub thread used by the transient demo flow. The id is provided by
 * the caller; if it's not already `local-thread:`-prefixed the caller
 * is responsible for understanding why (the in-flight new-thread flow
 * mints its own real UUID before the server responds).
 */
export function createStubThread(id: string = newLocalThreadId()): Thread {
	const now = new Date().toISOString();
	return {
		id,
		user_id: '',
		title: '',
		created_at: now,
		updated_at: now,
	};
}
