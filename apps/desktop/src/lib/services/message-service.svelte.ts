import { InjectionToken } from '@eurora/shared/context';
import type {
	MessageAssetChip,
	MessageTreeNodeView,
	MessageView,
	ResponseChunk,
	Query,
} from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';

const PAGE_SIZE = 50;

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

		function onEvent(response: ResponseChunk) {
			if (!agentMessage) {
				agentMessage = entry.messages.at(-1);
			}

			if (response.reasoning) {
				if (!entry.reasoningData[messageIndex]) {
					reasoningStartTime = Date.now();
					entry.reasoningData[messageIndex] = {
						content: response.reasoning,
						isStreaming: true,
					};
				} else {
					entry.reasoningData[messageIndex].content += response.reasoning;
				}
			}

			if (agentMessage && agentMessage.role === 'ai' && response.chunk) {
				if (!hasReceivedContent) {
					if (response.chunk.trim().length === 0) {
						pendingWhitespace += response.chunk;
						return;
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
				agentMessage.content += response.chunk;
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

	async loadTreeNodes(threadId: string, startLevel = 0, endLevel = 50): Promise<void> {
		const entry = this.cache.get(threadId);
		if (!entry) return;

		try {
			entry.treeNodes = await this.taurpc.thread.get_message_tree(
				threadId,
				startLevel,
				endLevel,
			);
		} catch (error) {
			console.error(`Failed to load tree nodes for thread ${threadId}:`, error);
		}
	}

	private refreshTreeIfNeeded(threadId: string): void {
		if (this.viewMode === 'graph') {
			this.loadTreeNodes(threadId);
		}
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
