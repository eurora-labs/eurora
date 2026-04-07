import { InjectionToken } from '@eurora/shared/context';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BranchDirection, IThreadService } from '$lib/services/thread/thread-service.js';

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
		const entry = this.threadIndex.get(threadId);
		if (!entry || entry.loading || entry.loaded || entry.streamingMessageId) return;

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

	async sendMessage(text: string, assetIds?: string[]): Promise<void> {
		if (!text.trim()) return;
		this.viewMode = 'list';

		let threadId = this.activeThreadId;
		let isNewThread = false;

		if (!threadId) {
			const thread = await this.threadClient.createThread();
			const entry = new ThreadMessages(thread);
			this.threads = [entry, ...this.threads];
			this.threadIndex.set(thread.id, entry);
			this.activeThreadId = thread.id;
			threadId = thread.id;
			this.newThread = thread;
			isNewThread = true;
		}

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		this.appendPlaceholders(entry, text);

		const onFirstChunk = isNewThread
			? () => {
					this.threadClient.generateTitle(threadId!, text).then((updated) => {
						this.updateThread(updated);
					});
				}
			: undefined;

		const receivedFinal = await this.consumeStream(
			entry,
			threadId,
			text,
			undefined,
			assetIds,
			onFirstChunk,
		);

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}
	}

	async editMessage(messageId: string, text: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;
		this.viewMode = 'list';

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		const nodeIndex = entry.messages.findIndex((n) => n.message.id === messageId);
		if (nodeIndex < 0) return;

		const parentId = entry.messages[nodeIndex].parentId;

		entry.messages = entry.messages.slice(0, nodeIndex);
		this.appendPlaceholders(entry, text);
		const receivedFinal = await this.consumeStream(entry, threadId, text, parentId);

		if (!receivedFinal) {
			await this.reconcileMessages(entry, threadId);
		}
		entry.invalidateFullTree();
	}

	private async reconcileMessages(entry: ThreadMessages, threadId: string): Promise<void> {
		const expectedCount = entry.messages.length;

		for (let attempt = 0; attempt < RECONCILE_RETRIES; attempt++) {
			await new Promise((resolve) => setTimeout(resolve, RECONCILE_DELAY_MS));

			const messages = await this.threadClient.getMessages(
				threadId,
				MESSAGE_PAGE_SIZE,
				0,
				false,
			);

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
		parentMessageId?: string | null,
		assetIds?: string[],
		onFirstChunk?: () => void,
	): Promise<boolean> {
		this.abortController?.abort();
		this.abortController = new AbortController();
		const { signal } = this.abortController;

		const aiNode = entry.messages.at(-1)!;
		const aiMessage = aiNode.message;
		if (aiMessage.type === 'remove') return false;

		let hasReceivedContent = false;
		let pendingWhitespace = '';
		let receivedFinal = false;

		try {
			for await (const event of this.threadClient.sendMessage(
				threadId,
				text,
				parentMessageId,
				signal,
				assetIds,
			)) {
				if (event.type === 'final') {
					entry.messages = event.messages;
					entry.loaded = true;
					entry.invalidateFullTree();
					receivedFinal = true;
					break;
				}

				const chunk = event.chunk;

				if (onFirstChunk) {
					onFirstChunk();
					onFirstChunk = undefined;
				}

				if (chunk.additionalKwargs) {
					try {
						const kwargs = JSON.parse(chunk.additionalKwargs);
						if (kwargs.reasoning_content) {
							const existing = aiMessage.content.find((b) => b.type === 'reasoning');
							if (existing && existing.type === 'reasoning') {
								existing.reasoning =
									(existing.reasoning ?? '') + kwargs.reasoning_content;
							} else {
								aiMessage.content.push({
									type: 'reasoning',
									id: null,
									reasoning: kwargs.reasoning_content,
									index: null,
									extras: null,
								});
							}
						}
					} catch {
						// Ignore malformed additional_kwargs JSON
					}
				}

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

	private appendPlaceholders(entry: ThreadMessages, text: string): void {
		const humanId = `temp-${crypto.randomUUID()}`;
		const aiId = `temp-${crypto.randomUUID()}`;

		entry.messages = [
			...entry.messages,
			{
				parentId: null,
				message: {
					type: 'human',
					content: [
						{
							type: 'text',
							id: null,
							text,
							annotations: [],
							index: null,
							extras: null,
						},
					],
					id: humanId,
					name: null,
					additionalKwargs: null,
					responseMetadata: null,
				},
				children: [],
				siblingIndex: 0,
				depth: 0,
			},
			{
				parentId: humanId,
				message: {
					type: 'ai',
					content: [],
					id: aiId,
					name: null,
					toolCalls: [],
					invalidToolCalls: [],
					usageMetadata: null,
					additionalKwargs: null,
					responseMetadata: null,
				},
				children: [],
				siblingIndex: 0,
				depth: 0,
			},
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
		nodeMap.set(msg.message.id, { ...msg, children: [] });
	}

	const roots: MessageNode[] = [];
	for (const msg of messages) {
		const node = nodeMap.get(msg.message.id)!;
		const parent = msg.parentId ? nodeMap.get(msg.parentId) : undefined;
		if (parent) {
			parent.children.push(node);
		} else {
			roots.push(node);
		}
	}

	return roots;
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
