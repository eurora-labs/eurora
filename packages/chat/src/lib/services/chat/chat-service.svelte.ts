import { DEFAULT_MODELS } from '$lib/models/chat-model.js';
import {
	AiStreamSink,
	createAiPlaceholderNode,
	createHumanPlaceholderNode,
	createLocalAiNode,
	createLocalHumanNode,
	createStubThread,
	isHumanNode,
	readAssetChips,
} from '$lib/models/messages/index.js';
import { InjectionToken } from '@eurora/shared/context';
import { SvelteMap } from 'svelte/reactivity';
import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type {
	AiPlaceholderNode,
	AssetChip,
	HumanPlaceholderNode,
	MessageNode,
} from '$lib/models/messages/index.js';
import type { ChatServerMessage } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BranchDirection, IThreadService } from '$lib/services/thread/thread-service.js';
import type { ChatSendRequest } from '@eurora/shared/bindings/thread';

const PAGE_SIZE = 20;
const MESSAGE_PAGE_SIZE = 50;
const RECONCILE_RETRIES = 3;
const RECONCILE_DELAY_MS = 1000;

/**
 * Snapshot of the host's "currently focused activity" at the moment a
 * thread mutation is about to happen — returns the activity uuid, or
 * undefined when no host context is available (web, mobile, or a desktop
 * client that hasn't recorded an activity yet).
 *
 * The provider is invoked at call time (not stored at construction time)
 * so the service always sees the *live* activity, not the user's
 * scrolled-to selection in the timeline rail.
 */
export type ActivityContextProvider = () => string | undefined;

export class ThreadMessages {
	// Explicit casting is fine because it's initialized in the constructor
	thread: Thread = $state() as Thread;
	messages: MessageNode[] = $state([]);
	loading = $state(false);
	hasMore = $state(true);
	offset = $state(0);
	streamingMessageId: string | null = $state(null);
	loaded = $state(false);
	isTransient = $state(false);

	constructor(thread: Thread) {
		this.thread = thread;
	}
}

export class ChatService {
	newThread: Thread | undefined = $state();

	threads: ThreadMessages[] = $state([]);
	activeThreadId: string | undefined = $state(undefined);
	activeThread = $derived(
		this.activeThreadId ? this.getThreadData(this.activeThreadId) : undefined,
	);
	loadingThreads = $state(false);
	loadingMoreThreads = $state(false);
	hasMoreThreads = $state(true);

	/**
	 * Per-activity thread buckets, populated on demand by
	 * [`loadThreadsForActivity`]. Keyed by `activity_id` (UUID string).
	 *
	 * Entries are reused across the lifetime of the service — the rail
	 * only surfaces 20 activities at a time, so the cache is bounded.
	 * The lazy population means typing a chat while the user is *not*
	 * actively browsing the rail costs nothing extra; the rail's
	 * filter pays only the activities it visits.
	 *
	 * Bucket entries are the same `ThreadMessages` instances as the
	 * matching rows in [`threads`] (via the [`threadIndex`] dedupe),
	 * so message-list state, loading flags, and streaming progress
	 * stay in sync regardless of which view the user opened a thread
	 * from.
	 */
	threadsByActivity: SvelteMap<string, ThreadMessages[]> = new SvelteMap();

	// Per-conversation behaviour flags. They live on the service rather than
	// inside the prompt-input component so they survive thread navigation and
	// can be read by `sendMessage` (and any future regenerate/edit paths).
	// NOTE: not yet forwarded to the backend — `ChatSendRequest` has no
	// matching fields. Wiring those requires a Specta regeneration pass.
	searchEnabled = $state(true);
	thinkingEnabled = $state(true);
	selectedModelId: string | undefined = $state(DEFAULT_MODELS[0]?.id);

	private readonly threadClient: IThreadService;
	private readonly activityContextProvider: ActivityContextProvider | undefined;

	private threadIndex = new Map<string, ThreadMessages>();
	private offset = 0;
	private activityLoadInFlight = new Set<string>();
	abortController: AbortController | null = null;

	constructor(threadClient: IThreadService, activityContextProvider?: ActivityContextProvider) {
		this.threadClient = threadClient;
		this.activityContextProvider = activityContextProvider;
	}

	async loadThreads(limit: number, offset: number) {
		this.loadingThreads = true;
		try {
			const fresh = await this.threadClient.listThreads(limit, offset);
			this.threads = fresh.map(
				(thread) => this.threadIndex.get(thread.id) ?? new ThreadMessages(thread),
			);
			this.rebuildIndex();
			this.offset = this.threads.length;
			this.hasMoreThreads = fresh.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loadingThreads = false;
		}
	}

	async loadMoreThreads() {
		if (this.loadingMoreThreads || !this.hasMoreThreads) return;
		this.loadingMoreThreads = true;
		try {
			const res = await this.threadClient.listThreads(PAGE_SIZE, this.offset);
			const newThreads = res.map((thread) => new ThreadMessages(thread));
			this.threads = [...this.threads, ...newThreads];
			this.rebuildIndex();
			this.offset += newThreads.length;
			this.hasMoreThreads = newThreads.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load more threads:', error);
		} finally {
			this.loadingMoreThreads = false;
		}
	}

	async deleteThread(threadId: string) {
		await this.threadClient.deleteThread(threadId);
		this.threads = this.threads.filter((t) => t.thread.id !== threadId);
		this.threadIndex.delete(threadId);
		this.offset = Math.max(0, this.offset - 1);
		if (this.activeThreadId === threadId) {
			this.activeThreadId = undefined;
		}
		// CASCADE handles the DB side; sweep the in-memory mirror so the
		// per-activity filter doesn't keep showing the deleted row.
		for (const [activityId, bucket] of this.threadsByActivity) {
			const filtered = bucket.filter((t) => t.thread.id !== threadId);
			if (filtered.length !== bucket.length) {
				this.threadsByActivity.set(activityId, filtered);
			}
		}
	}

	/**
	 * Populate [`threadsByActivity`] with the threads linked to
	 * `activityId`. Idempotent — repeat calls for the same id return
	 * immediately whether the bucket is already loaded or another fetch
	 * is in flight. Failures are logged and leave the bucket empty so
	 * the next call retries.
	 *
	 * Reuses cached `ThreadMessages` instances from [`threadIndex`] so
	 * the per-activity view and the full thread list share message
	 * state, loading flags, and streaming progress for the same thread.
	 */
	async loadThreadsForActivity(activityId: string): Promise<void> {
		if (this.threadsByActivity.has(activityId)) return;
		if (this.activityLoadInFlight.has(activityId)) return;

		this.activityLoadInFlight.add(activityId);
		try {
			const threads = await this.threadClient.listThreadsForActivity(activityId);
			const bucket = threads.map((thread) => {
				const existing = this.threadIndex.get(thread.id);
				if (existing !== undefined) {
					existing.thread = { ...existing.thread, ...thread };
					return existing;
				}
				const entry = new ThreadMessages(thread);
				this.threadIndex.set(thread.id, entry);
				return entry;
			});
			this.threadsByActivity.set(activityId, bucket);
		} catch (error) {
			console.error(`Failed to load threads for activity ${activityId}:`, error);
		} finally {
			this.activityLoadInFlight.delete(activityId);
		}
	}

	updateThread(thread: Thread) {
		const entry = this.threadIndex.get(thread.id);
		if (entry) {
			entry.thread = { ...entry.thread, ...thread };
		}
	}

	getThreadData(threadId: string | undefined): ThreadMessages | undefined {
		if (!threadId) return undefined;
		return this.threadIndex.get(threadId);
	}

	async loadMessages(threadId: string): Promise<void> {
		let entry = this.threadIndex.get(threadId);
		if (!entry) {
			entry = new ThreadMessages(createStubThread(threadId));
			this.threadIndex.set(threadId, entry);
		}
		if (entry.loading || entry.loaded || entry.streamingMessageId) return;

		entry.loading = true;
		try {
			const messages = await this.threadClient.getMessages(threadId, MESSAGE_PAGE_SIZE, 0);
			entry.messages = messages;
			entry.offset = messages.length;
			entry.hasMore = messages.length === MESSAGE_PAGE_SIZE;
			entry.loaded = true;
		} catch (error) {
			console.error(`Failed to load messages for thread ${threadId}:`, error);
		} finally {
			entry.loading = false;
		}
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<void> {
		const entry = this.threadIndex.get(threadId);
		if (!entry) return;

		const messages = await this.threadClient.switchBranch(threadId, messageId, direction);
		entry.messages = messages;
	}

	async sendMessage(text: string, assetChips: AssetChip[] = []): Promise<void> {
		if (!text.trim()) return;

		// Snapshot the live activity once at entry — the request carries
		// the link target and the in-memory cache mirrors it. Reading
		// inside `buildSendRequest` would be functionally equivalent today
		// but invites drift if the placeholder/stream timeline grows.
		const activityId = this.activityContextProvider?.();

		const current = this.activeThreadId;
		const existing = current ? this.threadIndex.get(current) : undefined;
		const needsNewThread = !current || existing?.isTransient === true;

		let threadId: string;

		if (needsNewThread) {
			if (current && existing?.isTransient) {
				this.threadIndex.delete(current);
			}
			const thread = await this.threadClient.createThread();
			const entry = new ThreadMessages(thread);
			this.threads = [entry, ...this.threads];
			this.threadIndex.set(thread.id, entry);
			this.activeThreadId = thread.id;
			threadId = thread.id;
			this.newThread = thread;
		} else {
			threadId = current!;
		}

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		const { humanNode, sink } = this.appendPlaceholders(entry, text, assetChips);

		const receivedFinal = await this.consumeStream(entry, threadId, text, humanNode, sink, {
			assetChips,
			activityId,
		});

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}

		if (activityId !== undefined) {
			this.linkActivityInCache(activityId, entry);
		}
	}

	addLocalExchange(userText: string, aiText: string): void {
		const thread = createStubThread();
		const entry = new ThreadMessages(thread);
		entry.isTransient = true;
		entry.loaded = true;

		const humanNode = createLocalHumanNode(null, userText);
		const aiNode = createLocalAiNode(humanNode.message.id, aiText);

		entry.messages = [humanNode, aiNode];

		this.threadIndex.set(thread.id, entry);
		this.activeThreadId = thread.id;
	}

	async regenerateAi(aiMessageId: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;

		const entry = this.getThreadData(threadId);
		if (!entry || entry.isTransient) return;

		const targetIndex = entry.messages.findIndex((n) => n.message.id === aiMessageId);
		if (targetIndex < 0) return;

		const target = entry.messages[targetIndex];
		const parentId = target.parent_id ?? null;
		if (target.message.type !== 'ai' || !parentId) return;

		this.abortController?.abort();

		const placeholder = createAiPlaceholderNode(parentId, '');
		entry.messages = entry.messages.map((node, i) => (i === targetIndex ? placeholder : node));
		// Construct the sink *after* the array assignment so the
		// resolver hits the reactive (proxied) view of the node, not the
		// raw reference. See `findStreamingPlaceholder`'s doc comment.
		const placeholderId = placeholder.message.id;
		const sink = new AiStreamSink(placeholderId, () =>
			findStreamingPlaceholder(entry, placeholderId),
		);
		entry.streamingMessageId = sink.id;

		const abortController = new AbortController();
		this.abortController = abortController;

		const stream = this.threadClient.regenerateAi(
			threadId,
			aiMessageId,
			abortController.signal,
		);
		const receivedFinal = await this.consumeChatStream(entry, stream, {
			sink,
			onConfirmedHumanMessage: null,
		});

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		} else {
			// The new AI variant is now the active leaf; refresh so the
			// branch tree picks up the new sibling counts and indices.
			try {
				const messages = await this.threadClient.getMessages(
					threadId,
					MESSAGE_PAGE_SIZE,
					0,
				);
				entry.messages = messages;
			} catch (error) {
				console.error(`Failed to refresh after regenerate for thread ${threadId}:`, error);
			}
		}
	}

	async editMessage(messageId: string, text: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;

		const entry = this.getThreadData(threadId);
		if (!entry || entry.isTransient) return;

		const nodeIndex = entry.messages.findIndex((n) => n.message.id === messageId);
		if (nodeIndex < 0) return;

		const original = entry.messages[nodeIndex];
		const parentId = original.parent_id ?? null;
		const preservedAssetChips = isHumanNode(original) ? readAssetChips(original.message) : [];

		// An edit is a fresh turn — link to whichever activity the user is
		// actually in *now*, not the one tagged on the original message.
		const activityId = this.activityContextProvider?.();

		entry.messages = entry.messages.slice(0, nodeIndex);
		const { humanNode, sink } = this.appendPlaceholders(entry, text, preservedAssetChips);
		const receivedFinal = await this.consumeStream(entry, threadId, text, humanNode, sink, {
			parentMessageId: parentId,
			preservedAssetChips,
			activityId,
		});

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}

		if (activityId !== undefined) {
			this.linkActivityInCache(activityId, entry);
		}
	}

	/**
	 * Optimistically mirror a thread→activity link in the in-memory
	 * [`threadsByActivity`] cache. The backend writes the row through
	 * `link_activity_to_thread`; this keeps the sidebar's filter view
	 * consistent without waiting for a refetch.
	 *
	 * No-op when the bucket has not been loaded yet (the next visit
	 * to that activity will rehydrate from the server) or when the
	 * thread is already in the bucket.
	 */
	private linkActivityInCache(activityId: string, entry: ThreadMessages): void {
		const bucket = this.threadsByActivity.get(activityId);
		if (bucket === undefined) return;
		if (bucket.some((t) => t.thread.id === entry.thread.id)) return;
		this.threadsByActivity.set(activityId, [entry, ...bucket]);
	}

	private async reconcileMessages(entry: ThreadMessages, threadId: string): Promise<void> {
		const expectedCount = entry.messages.length;

		for (let attempt = 0; attempt < RECONCILE_RETRIES; attempt++) {
			await new Promise((resolve) => setTimeout(resolve, RECONCILE_DELAY_MS));

			if (entry.streamingMessageId) return;

			const messages = await this.threadClient.getMessages(threadId, MESSAGE_PAGE_SIZE, 0);

			if (entry.streamingMessageId) return;

			if (messages.length >= expectedCount) {
				entry.messages = messages;
				return;
			}
		}
	}

	/**
	 * Shared chat-stream consumer. Drains the wire envelopes from `stream`
	 * into `entry`: routes streaming chunks through the sink, applies
	 * server-pushed title updates, and swaps the placeholder for the
	 * persisted message on `final`. Returns `true` iff a `final` frame was
	 * observed; callers reconcile against the server when the stream ends
	 * without one (cancellation, error, transport drop).
	 *
	 * The discriminator is matched exhaustively — any future
	 * [`ChatServerMessage`] variant will fail the build at the `default`
	 * arm rather than silently dropping into a fallthrough.
	 *
	 * Per-turn divergence between `send` and `regenerate` is funnelled
	 * through `policy.onConfirmedHumanMessage`: `send` uses the hook to
	 * swap the temp human node and bind the abort controller;
	 * `regenerate` passes `null` because no new human message exists, so
	 * stray envelopes are ignored.
	 */
	private async consumeChatStream(
		entry: ThreadMessages,
		stream: AsyncIterable<ChatServerMessage>,
		policy: ChatStreamPolicy,
	): Promise<boolean> {
		const { sink } = policy;
		let receivedFinal = false;

		try {
			consume: for await (const event of stream) {
				switch (event.type) {
					case 'confirmed_human_message':
						policy.onConfirmedHumanMessage?.(event.message);
						break;
					case 'chunk':
						sink.appendChunk(event.chunk);
						break;
					case 'title_updated':
						// The backend's agent loop auto-titles untitled threads
						// at the end of every turn and emits this frame before
						// the terminal `final`/`error` (or before dropping the
						// socket on cancel). Mutating the thread in place lets
						// the sidebar's Svelte `$state` binding repaint without
						// an extra HTTP round trip.
						this.updateThread({ ...entry.thread, title: event.title });
						break;
					case 'final': {
						const aiMsg = event.messages[0];
						if (aiMsg) {
							entry.messages = entry.messages.map((node) =>
								node.message.id === sink.id ? aiMsg : node,
							);
						}
						entry.loaded = true;
						receivedFinal = true;
						break consume;
					}
					case 'error':
						throw new Error(`${event.kind}: ${event.message}`);
					case 'tool_request':
					case 'tool_cancel': {
						// Client-side tool dispatch isn't wired up yet. The
						// backend's `ChatRemoteBus` already emits these frames;
						// until the client grows a dispatcher we drop them so
						// the chat stream still finishes cleanly when a server
						// runs ahead of the client.
						break;
					}
					default: {
						const _exhaustive: never = event;
						void _exhaustive;
					}
				}
			}
		} catch (e) {
			console.error(`Chat stream error for thread ${entry.thread.id}:`, e);
			if (sink.isEmpty) {
				entry.messages = entry.messages.filter((n) => n.message.id !== sink.id);
			}
		} finally {
			entry.streamingMessageId = null;
			entry.loaded = true;
		}

		return receivedFinal;
	}

	private async consumeStream(
		entry: ThreadMessages,
		threadId: string,
		text: string,
		humanPlaceholder: HumanPlaceholderNode,
		sink: AiStreamSink,
		options: {
			parentMessageId?: string | null;
			assetChips?: AssetChip[];
			preservedAssetChips?: AssetChip[];
			activityId?: string | undefined;
		} = {},
	): Promise<boolean> {
		this.abortController?.abort();

		const humanPlaceholderId = humanPlaceholder.message.id;
		const abortController = new AbortController();
		const request = await this.buildSendRequest(threadId, text, options);
		const stream = this.threadClient.sendMessage(threadId, request, abortController.signal);

		return this.consumeChatStream(entry, stream, {
			sink,
			onConfirmedHumanMessage: (confirmed) => {
				entry.messages = entry.messages.map((node) => {
					if (node.message.id === humanPlaceholderId) return confirmed;
					if (node.parent_id === humanPlaceholderId) {
						return { ...node, parent_id: confirmed.message.id };
					}
					return node;
				});
				this.abortController = abortController;
			},
		});
	}

	private async buildSendRequest(
		threadId: string,
		text: string,
		options: {
			parentMessageId?: string | null;
			assetChips?: AssetChip[];
			preservedAssetChips?: AssetChip[];
			activityId?: string | undefined;
		},
	): Promise<ChatSendRequest> {
		const parent_message_id = options.parentMessageId ?? null;
		const isEdit = options.preservedAssetChips !== undefined;

		// On edits we replay the chips that were already attached to the
		// original turn — there is no fresh activity to sample. On new
		// turns with attached UI chips we ask the host for the chip set
		// to persist alongside the message. Hosts without a context
		// source return empty arrays.
		//
		// The LLM-facing prelude blocks (the strategy's `get_context()`
		// output) are pulled separately by the chat bridge on the Rust
		// side and shipped in the `CapabilityUpdate.system_blocks` wire
		// field — they never round-trip through the frontend.
		const collectedChips =
			!isEdit && (options.assetChips?.length ?? 0) > 0
				? (await this.threadClient.collectContext(threadId)).assetChips
				: [];

		const persistedChips = isEdit ? (options.preservedAssetChips ?? []) : collectedChips;

		const userBlock: ContentBlock = {
			type: 'text',
			id: null,
			text,
			annotations: null,
			index: null,
			extras: null,
		};

		return {
			content_blocks: [userBlock],
			parent_message_id,
			asset_chips_json: persistedChips.length > 0 ? JSON.stringify(persistedChips) : null,
			activity_id: options.activityId ?? null,
		};
	}

	private appendPlaceholders(
		entry: ThreadMessages,
		text: string,
		assetChips: AssetChip[] = [],
	): { humanNode: HumanPlaceholderNode; sink: AiStreamSink } {
		const humanNode = createHumanPlaceholderNode(null, text, assetChips);
		const aiNode = createAiPlaceholderNode(humanNode.message.id, '');
		entry.messages = [...entry.messages, humanNode, aiNode];
		// Resolver looks up the node back through `entry.messages` so
		// every sink mutation passes through Svelte's `$state` proxy.
		// See `findStreamingPlaceholder`'s doc comment.
		const aiId = aiNode.message.id;
		const sink = new AiStreamSink(aiId, () => findStreamingPlaceholder(entry, aiId));
		entry.streamingMessageId = sink.id;
		return { humanNode, sink };
	}

	destroy() {
		this.abortController?.abort();
		this.abortController = null;
		this.threads = [];
		this.threadIndex.clear();
		this.threadsByActivity.clear();
		this.activityLoadInFlight.clear();
		this.offset = 0;
		this.hasMoreThreads = true;
		this.loadingThreads = false;
		this.activeThreadId = undefined;
	}

	private rebuildIndex(): void {
		this.threadIndex = new Map(this.threads.map((t) => [t.thread.id, t]));
	}
}

/**
 * Locate the live `AiPlaceholderNode` for `id` inside `entry.messages`.
 * Returns `null` if no such node exists (cancellation, error rollback)
 * or if the matching node is no longer an AI placeholder (it was
 * swapped for the persisted message when `final` arrived).
 *
 * Routed through `entry.messages.find(...)`, the lookup goes through
 * Svelte's `$state` proxy — handing the result to `AiStreamSink`
 * ensures every mutation flows through the reactive view, so chunks
 * render incrementally instead of buffering until the final swap.
 */
function findStreamingPlaceholder(entry: ThreadMessages, id: string): AiPlaceholderNode | null {
	const node = entry.messages.find((n) => n.message.id === id);
	if (!node || node.message.type !== 'ai') return null;
	return node as AiPlaceholderNode;
}

interface ChatStreamPolicy {
	/**
	 * Drives streaming mutations on the AI placeholder. The sink's
	 * `id` is the swap target when `final` arrives.
	 */
	sink: AiStreamSink;
	/**
	 * Invoked when the server confirms the user message has been persisted.
	 * `send` swaps the temp human node and binds the abort controller from
	 * inside the callback. `regenerate` has no fresh human message to
	 * confirm and passes `null` — stray envelopes are then ignored.
	 */
	onConfirmedHumanMessage: ((confirmed: MessageNode) => void) | null;
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
