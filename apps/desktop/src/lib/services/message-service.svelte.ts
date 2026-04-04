import { InjectionToken } from '@eurora/shared/context';
import type {
	MessageAssetChip,
	MessageTreeNodeView,
	MessageView,
	ProtoAiMessageChunk,
	Query,
} from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const PAGE_SIZE = 50;
const TREE_LEVEL_PAGE_SIZE = 5;

interface ReasoningData {
	content: string;
	isStreaming: boolean;
	duration?: number;
}

export class ThreadMessages {
	messages: MessageView[] = $state([]);
	treeNodes: MessageTreeNodeView[] = $state([]);
	reasoningData: Record<number, ReasoningData> = $state({});
	loading = $state(false);
	hasMore = $state(true);
	offset = 0;
	streaming = $state(false);

	treeLoadedEndLevel = 0;
	treeLoading = $state(false);
	treeHasMore = $state(false);
	treeLoadId = 0;
	treeInitialLoaded = false;
}

export type ViewMode = 'list' | 'graph';

export class MessageService {
	viewMode: ViewMode = $state('list');
	viewModeVisible = $state(false);

	private cache: Map<string, ThreadMessages> = $state(new Map());
	private readonly taurpc: TaurpcService;
	private readonly unlisteners: Promise<() => void>[] = [];

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	init() {
		this.unlisteners.push(
			this.taurpc.thread.current_thread_changed.on((thread) => {
				if (thread.id) {
					this.ensureLoaded(thread.id);
				}
			}),
		);
	}

	getThread(threadId: string): ThreadMessages {
		this.ensureLoaded(threadId);
		return this.cache.get(threadId)!;
	}

	private ensureLoaded(threadId: string) {
		if (this.cache.has(threadId)) return;

		const entry = new ThreadMessages();
		entry.loading = true;
		this.cache.set(threadId, entry);

		this.taurpc.thread
			.get_messages(threadId, PAGE_SIZE, 0)
			.then((messages) => {
				if (entry.messages.length > 0) return;
				entry.messages = messages;
				entry.offset = messages.length;
				entry.hasMore = messages.length === PAGE_SIZE;
				this.extractReasoning(entry, messages, 0);
				this.refreshTreeIfNeeded(threadId);
			})
			.catch((error) => {
				console.error(`Failed to load messages for thread ${threadId}:`, error);
			})
			.finally(() => {
				entry.loading = false;
			});
	}

	async loadMore(threadId: string) {
		const entry = this.cache.get(threadId);
		if (!entry || entry.loading || !entry.hasMore) return;

		entry.loading = true;
		try {
			const messages = await this.taurpc.thread.get_messages(
				threadId,
				PAGE_SIZE,
				entry.offset,
			);
			const insertOffset = entry.messages.length;
			entry.messages = [...entry.messages, ...messages];
			entry.offset += messages.length;
			entry.hasMore = messages.length === PAGE_SIZE;
			this.extractReasoning(entry, messages, insertOffset);
		} catch (error) {
			console.error(`Failed to load more messages for thread ${threadId}:`, error);
		} finally {
			entry.loading = false;
		}
	}

	async sendMessage(
		threadId: string,
		query: Query,
		assetChips?: MessageAssetChip[],
	): Promise<void> {
		const entry = this.cache.get(threadId) ?? this.getThread(threadId);

		entry.messages.push({
			id: null,
			role: 'human',
			content: query.text,
			reasoning_blocks: null,
			sibling_count: 1,
			sibling_index: 0,
			assets: assetChips?.length ? assetChips : null,
		});

		entry.messages.push({
			id: null,
			role: 'ai',
			content: '',
			reasoning_blocks: null,
			sibling_count: 1,
			sibling_index: 0,
			assets: null,
		});

		const messageIndex = entry.messages.length - 1;
		let agentMessage: MessageView | undefined;
		let reasoningStartTime: number | null = null;
		let hasReceivedContent = false;
		entry.streaming = true;

		let pendingWhitespace = '';

		function onEvent(response: ProtoAiMessageChunk) {
			if (!agentMessage) {
				agentMessage = entry.messages.at(-1);
			}

			for (const block of response.content) {
				if (!block.block) continue;

				if ('Reasoning' in block.block) {
					const reasoning = block.block.Reasoning.reasoning;
					if (reasoning) {
						if (!entry.reasoningData[messageIndex]) {
							reasoningStartTime = Date.now();
							entry.reasoningData[messageIndex] = {
								content: reasoning,
								isStreaming: true,
							};
						} else {
							entry.reasoningData[messageIndex].content += reasoning;
						}
					}
				} else if ('Text' in block.block) {
					const text = block.block.Text.text;
					if (agentMessage && agentMessage.role === 'ai' && text) {
						if (!hasReceivedContent) {
							if (text.trim().length === 0) {
								pendingWhitespace += text;
								continue;
							}
							hasReceivedContent = true;
							if (pendingWhitespace) {
								agentMessage.content += pendingWhitespace;
								pendingWhitespace = '';
							}
							if (entry.reasoningData[messageIndex]?.isStreaming) {
								entry.reasoningData[messageIndex].isStreaming = false;
								entry.reasoningData[messageIndex].duration = reasoningStartTime
									? Math.ceil((Date.now() - reasoningStartTime) / 1000)
									: undefined;
							}
						}
						agentMessage.content += text;
					}
				}
			}
		}

		try {
			await this.taurpc.chat.send_query(threadId, onEvent, query);
		} finally {
			if (entry.reasoningData[messageIndex]?.isStreaming) {
				entry.reasoningData[messageIndex].isStreaming = false;
				entry.reasoningData[messageIndex].duration = reasoningStartTime
					? Math.ceil((Date.now() - reasoningStartTime) / 1000)
					: undefined;
			}
			entry.streaming = false;
		}

		const fresh = await this.taurpc.thread.get_messages(threadId, PAGE_SIZE, 0);
		entry.messages = fresh;
		entry.reasoningData = {};
		this.extractReasoning(entry, fresh, 0);
		this.refreshTreeIfNeeded(threadId);
	}

	async editMessage(
		threadId: string,
		editIndex: number,
		newText: string,
		parentMessageId: string | null,
		assetChips: MessageAssetChip[] = [],
	): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry) return;

		entry.messages = entry.messages.slice(0, editIndex);
		for (const key of Object.keys(entry.reasoningData)) {
			if (Number(key) >= editIndex) delete entry.reasoningData[Number(key)];
		}

		const query: Query = {
			text: newText,
			assets: assetChips.map((a) => a.id),
			parent_message_id: parentMessageId,
		};
		await this.sendMessage(threadId, query, assetChips);
	}

	async switchBranch(threadId: string, messageId: string, direction: number): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry) return;

		const messages = await this.taurpc.thread.switch_branch(threadId, messageId, direction);
		entry.messages = messages;
		entry.reasoningData = {};
		this.extractReasoning(entry, messages, 0);
		this.refreshTreeIfNeeded(threadId);
	}

	async navigateToMessage(threadId: string, messageId: string): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry) return;

		const messages = await this.taurpc.thread.switch_branch(threadId, messageId, 0);
		entry.messages = messages;
		entry.reasoningData = {};
		this.extractReasoning(entry, messages, 0);
		this.viewMode = 'list';
	}

	ensureTreeLoaded(threadId: string): void {
		const entry = this.cache.get(threadId);
		if (!entry || entry.treeInitialLoaded || entry.treeLoading) return;
		entry.treeInitialLoaded = true;
		this.loadTreeNodes(threadId);
	}

	async loadTreeNodes(
		threadId: string,
		startLevel = 0,
		endLevel = TREE_LEVEL_PAGE_SIZE - 1,
		parentNodeIds: string[] = [],
	): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry || entry.treeLoading) return;

		const loadId = ++entry.treeLoadId;
		entry.treeLoading = true;
		try {
			const response = await this.taurpc.thread.get_message_tree(
				threadId,
				startLevel,
				endLevel,
				parentNodeIds,
			);
			if (entry.treeLoadId !== loadId) return;
			if (startLevel === 0) {
				entry.treeNodes = response.nodes;
			} else {
				const existingIds = new Set(entry.treeNodes.map((n) => n.id));
				const newNodes = response.nodes.filter((n) => !existingIds.has(n.id));
				entry.treeNodes = [...entry.treeNodes, ...newNodes];
			}
			entry.treeLoadedEndLevel = endLevel;
			entry.treeHasMore = response.has_more;
		} catch (error) {
			if (entry.treeLoadId !== loadId) return;
			console.error(`Failed to load tree nodes for thread ${threadId}:`, error);
		} finally {
			if (entry.treeLoadId === loadId) {
				entry.treeLoading = false;
			}
		}
	}

	async loadMoreTreeLevels(threadId: string, count = TREE_LEVEL_PAGE_SIZE): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry || entry.treeLoading || !entry.treeHasMore) return;
		if (entry.treeNodes.length === 0) return;

		const maxLevel = entry.treeLoadedEndLevel;
		const boundaryIds = entry.treeNodes.filter((n) => n.level === maxLevel).map((n) => n.id);

		const startLevel = maxLevel + 1;
		const endLevel = startLevel + count - 1;
		await this.loadTreeNodes(threadId, startLevel, endLevel, boundaryIds);
	}

	private refreshTreeIfNeeded(threadId: string): void {
		if (this.viewMode !== 'graph') return;
		const entry = this.cache.get(threadId);
		if (!entry) return;
		entry.treeInitialLoaded = false;
		this.loadTreeNodes(
			threadId,
			0,
			Math.max(entry.treeLoadedEndLevel, TREE_LEVEL_PAGE_SIZE - 1),
		);
	}

	isStreaming(threadId: string): boolean {
		return this.cache.get(threadId)?.streaming ?? false;
	}

	clearThread(threadId: string) {
		this.cache.delete(threadId);
	}

	private extractReasoning(entry: ThreadMessages, messages: MessageView[], startIndex: number) {
		messages.forEach((msg, i) => {
			if (msg.reasoning_blocks?.length) {
				const content = msg.reasoning_blocks.map((b) => b.content ?? '').join('');
				if (content) {
					entry.reasoningData[startIndex + i] = { content, isStreaming: false };
				}
			}
		});
	}

	destroy() {
		for (const p of this.unlisteners) {
			p.then((unlisten) => unlisten());
		}
		this.unlisteners.length = 0;
		this.cache.clear();
	}
}

export const MESSAGE_SERVICE = new InjectionToken<MessageService>('MessageService');
