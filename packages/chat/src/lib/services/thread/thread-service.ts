import { InjectionToken } from '@eurora/shared/context';
import type { AssetChip, MessageNode } from '$lib/models/messages/index.js';
import type { MessageSearchResult, ThreadSearchResult } from '$lib/models/search.model.js';
import type { ChatServerMessage } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { ChatSendRequest } from '@eurora/shared/bindings/thread';

export type BranchDirection = -1 | 0 | 1;

/**
 * Per-turn UI metadata contributed by the host environment.
 *
 * On desktop this carries the active timeline activity's context chip;
 * on web it is empty. The chat service persists `assetChips` alongside
 * the user message via `ChatSendRequest.asset_chips_json`.
 *
 * The LLM-facing prelude (a short summary of what the user is doing,
 * authored by the active activity strategy) is delivered separately
 * over the chat WebSocket via the `system_blocks` field on the
 * `CapabilityUpdate` frame; the frontend never sees it.
 */
export interface ChatContext {
	assetChips: AssetChip[];
}

export interface IThreadService {
	listThreads(limit: number, offset: number): Promise<Thread[]>;
	/**
	 * List threads linked to a single timeline activity via the
	 * `activity_threads` junction. Powers the desktop sidebar's per-app
	 * filter; non-desktop hosts may implement as an empty result.
	 */
	listThreadsForActivity(activityId: string): Promise<Thread[]>;
	getMessages(threadId: string, limit: number, offset: number): Promise<MessageNode[]>;
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

	/// Open the chat WebSocket in regenerate mode. The server rewinds
	/// `active_leaf` to the AI message's human parent and runs the agent on
	/// the existing context; the new AI response lands as a sibling variant.
	regenerateAi(
		threadId: string,
		aiMessageId: string,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
