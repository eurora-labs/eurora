import { InjectionToken } from '@eurora/shared/context';
import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { AssetChip, MessageNode } from '$lib/models/messages/index.js';
import type { MessageSearchResult, ThreadSearchResult } from '$lib/models/search.model.js';
import type { ChatServerMessage } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { ChatSendRequest } from '@eurora/shared/bindings/thread';

export type BranchDirection = -1 | 0 | 1;

/**
 * Per-turn context contributed by the host environment.
 *
 * On desktop this is the active timeline activity (asset bytes, snapshot
 * blocks, the matching context chips); on web it is empty. The chat service
 * merges `contentBlocks` into the user's message and persists `assetChips`
 * alongside it via `ChatSendRequest.asset_chips_json`.
 */
export interface ChatContext {
	contentBlocks: ContentBlock[];
	assetChips: AssetChip[];
}

export interface IThreadService {
	listThreads(limit: number, offset: number): Promise<Thread[]>;
	getMessages(
		threadId: string,
		limit: number,
		offset: number,
		allVariants: boolean,
	): Promise<MessageNode[]>;
	switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]>;
	deleteThread(threadId: string): Promise<void>;
	createThread(): Promise<Thread>;
	generateTitle(threadId: string): Promise<Thread>;
	searchThreads(query: string, limit: number, offset: number): Promise<ThreadSearchResult[]>;
	searchMessages(query: string, limit: number, offset: number): Promise<MessageSearchResult[]>;

	/// Collect host-supplied chat context for a fresh turn. Implementations
	/// without a context source (web SPA, mobile) return empty arrays.
	collectContext(threadId: string): Promise<ChatContext>;

	/// Open the chat WebSocket and stream `ChatServerMessage` envelopes back
	/// to the caller. Cancellation via `signal` triggers a graceful close.
	sendMessage(
		threadId: string,
		request: ChatSendRequest,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
