import { InjectionToken } from '@eurora/shared/context';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BranchDirection, IThreadService } from '$lib/services/thread/thread-service.js';

export type ViewMode = 'list' | 'graph';

const PAGE_SIZE = 20;
const MESSAGE_PAGE_SIZE = 50;
const TREE_INITIAL_DEPTH = 5;
const TREE_LEVEL_PAGE_SIZE = 5;

export class ThreadMessages {
	thread: Thread;
	messages: MessageNode[] = $state([]);
	loading = $state(false);
	hasMore = $state(true);
	offset = $state(0);
	streamingMessageId: string | null = $state(null);
	loaded = $state(false);

	treeRoots: MessageNode[] = $state([]);
	treeLoading = $state(false);
	treeLoaded = $state(false);
	treeHasMore = $state(false);
	treeLoadedEndLevel = 0;

	constructor(thread: Thread) {
		this.thread = thread;
	}

	invalidateTree(): void {
		this.treeLoaded = false;
		this.treeRoots = [];
		this.treeLoadedEndLevel = 0;
		this.treeHasMore = false;
	}
}

export class ChatService {
	newThread: Thread | undefined = $state();
	titleChanged: Thread | undefined = $state();

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

	private offset = 0;
	private abortController: AbortController | null = null;
	private readonly unlisteners: ((() => void) | Promise<() => void>)[] = [];

	constructor(threadClient: IThreadService) {
		this.threadClient = threadClient;
	}

	async loadThreads(limit: number, offset: number) {
		try {
			const fresh = await this.threadClient.listThreads(limit, offset);
			const existing = new Map(this.threads.map((t) => [t.thread.id, t]));
			this.threads = fresh.map(
				(thread) => existing.get(thread.id) ?? new ThreadMessages(thread),
			);
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
		this.offset = Math.max(0, this.offset - 1);
		if (this.activeThreadId === threadId) {
			this.activeThreadId = undefined;
		}
	}

	updateThread(thread: Thread) {
		const entry = this.threads.find((t) => t.thread.id === thread.id);
		if (entry) {
			entry.thread = { ...entry.thread, ...thread };
		}
	}

	getThreadData(threadId: string | undefined): ThreadMessages | undefined {
		if (!threadId) return undefined;
		return this.threads.find((t) => t.thread.id === threadId);
	}

	async loadMessages(threadId: string): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry || entry.loading || entry.loaded || entry.streamingMessageId) return;

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
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry) return;

		const messages = await this.threadClient.switchBranch(threadId, messageId, direction);
		entry.messages = messages;
		entry.invalidateTree();
	}

	async loadTree(threadId: string): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry || entry.treeLoading || entry.treeLoaded) return;

		entry.treeLoading = true;
		try {
			const res = await this.threadClient.getMessageTree(
				threadId,
				0,
				TREE_INITIAL_DEPTH - 1,
				[],
			);
			entry.treeRoots = res.roots;
			entry.treeHasMore = res.hasMore;
			entry.treeLoadedEndLevel = TREE_INITIAL_DEPTH - 1;
			entry.treeLoaded = true;
		} catch (error) {
			console.error(`Failed to load tree for thread ${threadId}:`, error);
		} finally {
			entry.treeLoading = false;
		}
	}

	async loadMoreTreeLevels(threadId: string): Promise<void> {
		const entry = this.threads.find((t) => t.thread.id === threadId);
		if (!entry || entry.treeLoading || !entry.treeHasMore || entry.treeRoots.length === 0)
			return;

		const maxLevel = entry.treeLoadedEndLevel;
		const boundaryIds = collectNodeIdsAtDepth(entry.treeRoots, maxLevel);

		const startLevel = maxLevel + 1;
		const endLevel = startLevel + TREE_LEVEL_PAGE_SIZE - 1;

		entry.treeLoading = true;
		try {
			const res = await this.threadClient.getMessageTree(
				threadId,
				startLevel,
				endLevel,
				boundaryIds,
			);
			graftSubtrees(entry.treeRoots, res.roots);
			entry.treeRoots = [...entry.treeRoots];
			entry.treeHasMore = res.hasMore;
			entry.treeLoadedEndLevel = endLevel;
		} catch (error) {
			console.error(`Failed to load more tree levels for thread ${threadId}:`, error);
		} finally {
			entry.treeLoading = false;
		}
	}

	async sendMessage(text: string): Promise<void> {
		if (!text.trim()) return;

		let threadId = this.activeThreadId;

		if (!threadId) {
			const thread = await this.threadClient.createThread();
			const entry = new ThreadMessages(thread);
			this.threads = [entry, ...this.threads];
			this.activeThreadId = thread.id;
			threadId = thread.id;
			this.newThread = thread;
		}

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		this.appendPlaceholders(entry, text);
		await this.consumeStream(entry, threadId, text);
	}

	async editMessage(messageId: string, text: string): Promise<void> {
		const threadId = this.activeThreadId;
		if (!threadId) return;

		const entry = this.getThreadData(threadId);
		if (!entry) return;

		const nodeIndex = entry.messages.findIndex((n) => n.message.id === messageId);
		if (nodeIndex < 0) return;

		const parentId = entry.messages[nodeIndex].parentId;

		entry.messages = entry.messages.slice(0, nodeIndex);
		this.appendPlaceholders(entry, text);
		await this.consumeStream(entry, threadId, text, parentId);

		const messages = await this.threadClient.getMessages(threadId, MESSAGE_PAGE_SIZE, 0);
		entry.messages = messages;
		entry.invalidateTree();
	}

	private async consumeStream(
		entry: ThreadMessages,
		threadId: string,
		text: string,
		parentMessageId?: string | null,
	): Promise<void> {
		this.abortController?.abort();
		this.abortController = new AbortController();
		const { signal } = this.abortController;

		const aiNode = entry.messages.at(-1)!;
		const aiMessage = aiNode.message;
		if (aiMessage.type === 'remove') return;

		let hasReceivedContent = false;
		let pendingWhitespace = '';

		try {
			for await (const event of this.threadClient.sendMessage(
				threadId,
				text,
				parentMessageId,
				signal,
			)) {
				if (event.type === 'final') {
					entry.messages = event.messages;
					entry.loaded = true;
					entry.invalidateTree();
					break;
				}

				const chunk = event.chunk;

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
		} finally {
			entry.streamingMessageId = null;
			if (!entry.loaded) {
				entry.loaded = true;
			}
		}
	}

	private appendPlaceholders(entry: ThreadMessages, text: string): void {
		const humanId = `temp-${crypto.randomUUID()}`;
		const aiId = `temp-${crypto.randomUUID()}`;

		entry.messages = [
			...entry.messages,
			{
				parentId: '',
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
		for (const p of this.unlisteners) {
			if (p instanceof Promise) {
				p.then((unlisten) => unlisten());
			} else {
				p();
			}
		}
		this.unlisteners.length = 0;
		this.threads = [];
		this.offset = 0;
		this.hasMoreThreads = true;
		this.loadingThreads = false;
		this.activeThreadId = undefined;
	}
}

function collectNodeIdsAtDepth(roots: MessageNode[], depth: number): string[] {
	const ids: string[] = [];
	function walk(nodes: MessageNode[]): void {
		for (const node of nodes) {
			if (node.depth === depth) {
				ids.push(node.message.id);
			} else if (node.depth < depth) {
				walk(node.children);
			}
		}
	}
	walk(roots);
	return ids;
}

function graftSubtrees(existingRoots: MessageNode[], newRoots: MessageNode[]): void {
	const parentIndex = new Map<string, MessageNode>();
	function index(nodes: MessageNode[]): void {
		for (const node of nodes) {
			parentIndex.set(node.message.id, node);
			index(node.children);
		}
	}
	index(existingRoots);

	for (const newRoot of newRoots) {
		if (newRoot.parentId) {
			const parent = parentIndex.get(newRoot.parentId);
			if (parent) {
				parent.children.push(newRoot);
			}
		}
	}
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
