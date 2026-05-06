import { InjectionToken } from '@eurora/shared/context';
import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { AssetChip, MessageNode } from '$lib/models/messages/index.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BranchDirection, IThreadService } from '$lib/services/thread/thread-service.js';
import type { ChatSendRequest } from '@eurora/shared/bindings/thread';

export type ViewMode = 'list' | 'graph';

const PAGE_SIZE = 20;
const MESSAGE_PAGE_SIZE = 50;
const RECONCILE_RETRIES = 3;
const RECONCILE_DELAY_MS = 1000;

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

	fullTree: MessageNode[] | null = $state(null);
	fullTreeLoading = $state(false);

	treeRoots: MessageNode[] = $derived(this.fullTree ?? buildTreeFromBranch(this.messages));

	constructor(thread: Thread) {
		this.thread = thread;
	}

	invalidateFullTree(): void {
		this.fullTree = null;
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
	viewMode: ViewMode = $state('list');

	private readonly threadClient: IThreadService;

	private threadIndex = new Map<string, ThreadMessages>();
	private offset = 0;
	abortController: AbortController | null = null;

	constructor(threadClient: IThreadService) {
		this.threadClient = threadClient;
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
			entry = new ThreadMessages(makeStubThread(threadId));
			this.threadIndex.set(threadId, entry);
		}
		if (entry.loading || entry.loaded || entry.streamingMessageId) return;

		entry.loading = true;
		try {
			const messages = await this.threadClient.getMessages(
				threadId,
				MESSAGE_PAGE_SIZE,
				0,
				false,
			);
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
		entry.invalidateFullTree();
	}

	async loadFullTree(threadId: string): Promise<void> {
		const entry = this.threadIndex.get(threadId);
		if (!entry || entry.fullTreeLoading || entry.fullTree) return;

		entry.fullTreeLoading = true;
		try {
			const roots = await this.threadClient.getMessages(threadId, 0, 0, true);
			entry.fullTree = roots;
		} catch (error) {
			console.error(`Failed to load full tree for thread ${threadId}:`, error);
		} finally {
			entry.fullTreeLoading = false;
		}
	}

	async sendMessage(text: string, assetChips: AssetChip[] = []): Promise<void> {
		if (!text.trim()) return;
		this.viewMode = 'list';

		const current = this.activeThreadId;
		const existing = current ? this.threadIndex.get(current) : undefined;
		const needsNewThread = !current || existing?.isTransient === true;

		let threadId: string;
		let isNewThread = false;

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
			isNewThread = true;
		} else {
			threadId = current!;
		}

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		this.appendPlaceholders(entry, text, assetChips);

		const onFirstChunk = isNewThread
			? () => {
					this.threadClient.generateTitle(threadId).then((updated) => {
						this.updateThread(updated);
					});
				}
			: undefined;

		const receivedFinal = await this.consumeStream(entry, threadId, text, {
			assetChips,
			onFirstChunk,
		});

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}
	}

	addLocalExchange(userText: string, aiText: string): void {
		this.viewMode = 'list';

		const threadId = `transient-${crypto.randomUUID()}`;
		const entry = new ThreadMessages(makeStubThread(threadId));
		entry.isTransient = true;
		entry.loaded = true;

		const humanId = `local-${crypto.randomUUID()}`;
		const aiId = `local-${crypto.randomUUID()}`;

		entry.messages = [
			makeHumanNode(humanId, null, userText, []),
			makeAiNode(aiId, humanId, aiText),
		];

		this.threadIndex.set(threadId, entry);
		this.activeThreadId = threadId;
	}

	async editMessage(messageId: string, text: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;
		this.viewMode = 'list';

		const entry = this.getThreadData(threadId);
		if (!entry || entry.isTransient) return;

		const nodeIndex = entry.messages.findIndex((n) => n.message.id === messageId);
		if (nodeIndex < 0) return;

		const original = entry.messages[nodeIndex];
		const parentId = original.parent_id ?? null;
		const preservedAssetChips =
			original.message.type === 'human' ? extractAssetChips(original.message) : [];

		entry.messages = entry.messages.slice(0, nodeIndex);
		this.appendPlaceholders(entry, text, preservedAssetChips);
		const receivedFinal = await this.consumeStream(entry, threadId, text, {
			parentMessageId: parentId,
			preservedAssetChips,
		});

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}
		entry.invalidateFullTree();
	}

	private async reconcileMessages(entry: ThreadMessages, threadId: string): Promise<void> {
		const expectedCount = entry.messages.length;

		for (let attempt = 0; attempt < RECONCILE_RETRIES; attempt++) {
			await new Promise((resolve) => setTimeout(resolve, RECONCILE_DELAY_MS));

			if (entry.streamingMessageId) return;

			const messages = await this.threadClient.getMessages(
				threadId,
				MESSAGE_PAGE_SIZE,
				0,
				false,
			);

			if (entry.streamingMessageId) return;

			if (messages.length >= expectedCount) {
				entry.messages = messages;
				return;
			}
		}
	}

	private async consumeStream(
		entry: ThreadMessages,
		threadId: string,
		text: string,
		options: {
			parentMessageId?: string | null;
			assetChips?: AssetChip[];
			preservedAssetChips?: AssetChip[];
			onFirstChunk?: () => void;
		} = {},
	): Promise<boolean> {
		this.abortController?.abort();

		const aiNode = entry.messages.at(-1)!;
		const aiMessage = aiNode.message;
		if (aiMessage.type === 'remove') return false;

		const tempHumanId = entry.messages.at(-2)?.message.id;
		const tempAiId = aiMessage.id;

		let hasReceivedContent = false;
		let pendingWhitespace = '';
		let receivedFinal = false;

		const abortController = new AbortController();
		const request = await this.buildSendRequest(threadId, text, options);
		const stream = this.threadClient.sendMessage(threadId, request, abortController.signal);
		let onFirstChunk = options.onFirstChunk;

		try {
			for await (const event of stream) {
				if (event.type === 'confirmed_human_message') {
					const confirmed = event.message;
					entry.messages = entry.messages.map((node) => {
						if (node.message.id === tempHumanId) return confirmed;
						if (node.parent_id === tempHumanId)
							return { ...node, parent_id: confirmed.message.id };
						return node;
					});
					this.abortController = abortController;
					continue;
				}

				if (event.type === 'final') {
					const aiMsg = event.messages[0];
					if (aiMsg) {
						entry.messages = entry.messages.map((node) =>
							node.message.id === tempAiId ? aiMsg : node,
						);
					}
					entry.loaded = true;
					entry.invalidateFullTree();
					receivedFinal = true;
					break;
				}

				if (event.type === 'error') {
					throw new Error(`${event.kind}: ${event.message}`);
				}

				const chunk = event.chunk;

				if (onFirstChunk) {
					onFirstChunk();
					onFirstChunk = undefined;
				}

				// Mirror agent-chain's `extract_reasoning_from_additional_kwargs`:
				// providers like DeepSeek, Ollama, XAI emit reasoning in
				// additional_kwargs. Accumulate it on the AI message's kwargs
				// (the wire shape) so the UI can render it without inventing a
				// fake `reasoning` content block.
				appendReasoningKwargs(aiMessage, chunk);

				for (const block of chunk.content) {
					if (block.type === 'text') {
						let textContent = block.text;
						if (!hasReceivedContent) {
							if (textContent.trim().length === 0) {
								pendingWhitespace += textContent;
								continue;
							}
							hasReceivedContent = true;
							textContent = pendingWhitespace + textContent;
							pendingWhitespace = '';
						}
						const existing = aiMessage.content.find((b) => b.type === 'text');
						if (existing && existing.type === 'text') {
							existing.text += textContent;
						} else {
							aiMessage.content.push({ ...block, text: textContent });
						}
					} else if (block.type === 'reasoning') {
						const existing = aiMessage.content.find((b) => b.type === 'reasoning');
						if (existing && existing.type === 'reasoning') {
							existing.reasoning =
								(existing.reasoning ?? '') + (block.reasoning ?? '');
						} else {
							aiMessage.content.push({ ...block });
						}
					} else {
						aiMessage.content.push(block);
					}
				}
			}
		} catch (e) {
			console.error(`Stream error for thread ${threadId}:`, e);
			if ('content' in aiMessage && aiMessage.content.length === 0) {
				entry.messages = entry.messages.filter((n) => n.message.id !== aiMessage.id);
			}
		} finally {
			entry.streamingMessageId = null;
			if (!entry.loaded) {
				entry.loaded = true;
			}
		}

		return receivedFinal;
	}

	private async buildSendRequest(
		threadId: string,
		text: string,
		options: {
			parentMessageId?: string | null;
			assetChips?: AssetChip[];
			preservedAssetChips?: AssetChip[];
		},
	): Promise<ChatSendRequest> {
		const parent_message_id = options.parentMessageId ?? null;
		const isEdit = options.preservedAssetChips !== undefined;

		// On edits we replay the chips that were already attached to the
		// original turn — there is no fresh activity to sample. On new turns
		// with attached UI chips we ask the host to assemble per-turn context
		// (asset bytes, snapshots, the persisted chip). Hosts without a
		// context source return empty arrays.
		const context =
			!isEdit && (options.assetChips?.length ?? 0) > 0
				? await this.threadClient.collectContext(threadId)
				: { contentBlocks: [], assetChips: [] };

		const persistedChips = isEdit ? (options.preservedAssetChips ?? []) : context.assetChips;

		const userBlock: ContentBlock = {
			type: 'text',
			id: null,
			text,
			annotations: null,
			index: null,
			extras: null,
		};

		return {
			content_blocks: [...context.contentBlocks, userBlock],
			parent_message_id,
			asset_chips_json: persistedChips.length > 0 ? JSON.stringify(persistedChips) : null,
		};
	}

	private appendPlaceholders(
		entry: ThreadMessages,
		text: string,
		assetChips: AssetChip[] = [],
	): void {
		const humanId = `temp-${crypto.randomUUID()}`;
		const aiId = `temp-${crypto.randomUUID()}`;

		entry.messages = [
			...entry.messages,
			makeHumanNode(humanId, null, text, assetChips),
			makeAiNode(aiId, humanId, ''),
		];
		entry.streamingMessageId = aiId;
	}

	destroy() {
		this.abortController?.abort();
		this.abortController = null;
		this.threads = [];
		this.threadIndex.clear();
		this.offset = 0;
		this.hasMoreThreads = true;
		this.loadingThreads = false;
		this.activeThreadId = undefined;
	}

	private rebuildIndex(): void {
		this.threadIndex = new Map(this.threads.map((t) => [t.thread.id, t]));
	}
}

function buildTreeFromBranch(messages: MessageNode[]): MessageNode[] {
	if (messages.length === 0) return [];

	const nodeMap = new Map<string, MessageNode>();
	for (const msg of messages) {
		const id = msg.message.id ?? '';
		nodeMap.set(id, { ...msg, children: [] });
	}

	const roots: MessageNode[] = [];
	for (const msg of messages) {
		const id = msg.message.id ?? '';
		const node = nodeMap.get(id)!;
		const parent = msg.parent_id ? nodeMap.get(msg.parent_id) : undefined;
		if (parent) {
			parent.children = [...(parent.children ?? []), node];
		} else {
			roots.push(node);
		}
	}

	return roots;
}

function makeHumanNode(
	id: string,
	parentId: string | null,
	text: string,
	assetChips: AssetChip[],
): MessageNode {
	return {
		parent_id: parentId,
		message: {
			type: 'human',
			content:
				text.length > 0
					? [
							{
								type: 'text',
								text,
								id: null,
								annotations: null,
								index: null,
								extras: null,
							},
						]
					: [],
			id,
			name: null,
			additional_kwargs: assetChips.length > 0 ? { asset_chips: assetChips } : {},
			response_metadata: {},
		},
		children: [],
		sibling_index: 0,
		depth: 0,
	};
}

function makeAiNode(id: string, parentId: string | null, text: string): MessageNode {
	return {
		parent_id: parentId,
		message: {
			type: 'ai',
			content: text
				? [{ type: 'text', text, id: null, annotations: null, index: null, extras: null }]
				: [],
			id,
			name: null,
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

function makeStubThread(id: string): Thread {
	const now = new Date().toISOString();
	return {
		id,
		user_id: '',
		title: '',
		created_at: now,
		updated_at: now,
	} as Thread;
}

function appendReasoningKwargs(
	aiMessage: MessageNode['message'],
	chunk: { additional_kwargs?: unknown },
): void {
	if (aiMessage.type !== 'ai') return;
	const incoming = chunk.additional_kwargs;
	if (!isObject(incoming)) return;
	const reasoning =
		typeof incoming.reasoning_content === 'string' ? incoming.reasoning_content : null;
	if (reasoning === null || reasoning.length === 0) return;
	const kwargs = isObject(aiMessage.additional_kwargs)
		? (aiMessage.additional_kwargs as Record<string, unknown>)
		: {};
	const previous = typeof kwargs.reasoning_content === 'string' ? kwargs.reasoning_content : '';
	kwargs.reasoning_content = previous + reasoning;
	(aiMessage as { additional_kwargs: unknown }).additional_kwargs = kwargs;
}

function isObject(v: unknown): v is Record<string, unknown> {
	return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function extractAssetChips(message: MessageNode['message']): AssetChip[] {
	if (message.type !== 'human') return [];
	const kwargs = message.additional_kwargs;
	if (!isObject(kwargs)) return [];
	const raw = kwargs.asset_chips;
	if (!Array.isArray(raw)) return [];
	const chips: AssetChip[] = [];
	for (const entry of raw) {
		if (!isObject(entry)) continue;
		const id = typeof entry.id === 'string' ? entry.id : null;
		const name = typeof entry.name === 'string' ? entry.name : null;
		if (id === null || name === null) continue;
		chips.push({
			id,
			name,
			icon: typeof entry.icon === 'string' ? entry.icon : null,
			domain: typeof entry.domain === 'string' ? entry.domain : null,
		});
	}
	return chips;
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
