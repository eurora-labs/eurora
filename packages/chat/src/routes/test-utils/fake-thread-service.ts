import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { MessageSearchResult, ThreadSearchResult } from '$lib/models/search.model.js';
import type { ChatStreamEvent } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type {
	BranchDirection,
	IThreadService,
	SendMessageOptions,
} from '$lib/services/thread/thread-service.js';

let nextId = 1;

function makeThread(overrides?: Partial<Thread>): Thread {
	const id = `thread-${nextId++}`;
	return {
		id,
		title: `Thread ${id}`,
		createdAt: new Date().toISOString(),
		updatedAt: new Date().toISOString(),
		...overrides,
	};
}

export function makeMessageNode(
	id: string,
	type: 'human' | 'ai',
	text: string,
	overrides?: Partial<MessageNode>,
): MessageNode {
	const content: ContentBlock[] =
		text.length > 0
			? [{ type: 'text', id: null, text, annotations: [], index: null, extras: null }]
			: [];

	const base: MessageNode = {
		parentId: null,
		message:
			type === 'human'
				? {
						type: 'human',
						content,
						id,
						name: null,
						additionalKwargs: null,
						responseMetadata: null,
						assetChips: [],
					}
				: {
						type: 'ai',
						content,
						id,
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
	};
	return { ...base, ...overrides };
}

export function makeReasoningNode(id: string, reasoning: string, text: string): MessageNode {
	const content: ContentBlock[] = [
		{ type: 'reasoning', id: null, reasoning, index: null, extras: null },
	];
	if (text) {
		content.push({ type: 'text', id: null, text, annotations: [], index: null, extras: null });
	}
	return {
		parentId: null,
		message: {
			type: 'ai',
			content,
			id,
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
	};
}

export class FakeThreadService implements IThreadService {
	threads: Thread[] = [];
	messagesByThread = new Map<string, MessageNode[]>();
	branchResults = new Map<string, MessageNode[]>();

	deleteDelay = 0;
	shouldFailDelete = false;

	streamChunks: ChatStreamEvent[] = [];
	streamDelay = 0;

	seed(count: number): void {
		this.threads = Array.from({ length: count }, (_, i) =>
			makeThread({ title: `Chat ${i + 1}` }),
		);
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		return this.threads.slice(offset, offset + limit);
	}

	async getMessages(
		threadId: string,
		_limit: number,
		_offset: number,
		_allVariants: boolean,
	): Promise<MessageNode[]> {
		return this.messagesByThread.get(threadId) ?? [];
	}

	async switchBranch(
		threadId: string,
		_messageId: string,
		_direction: BranchDirection,
	): Promise<MessageNode[]> {
		return this.branchResults.get(threadId) ?? this.messagesByThread.get(threadId) ?? [];
	}

	async deleteThread(threadId: string): Promise<void> {
		if (this.deleteDelay > 0) {
			await new Promise((r) => setTimeout(r, this.deleteDelay));
		}
		if (this.shouldFailDelete) {
			throw new Error('Delete failed');
		}
		this.threads = this.threads.filter((t) => t.id !== threadId);
	}

	async createThread(): Promise<Thread> {
		const thread = makeThread();
		this.threads.unshift(thread);
		return thread;
	}

	async generateTitle(threadId: string, _content: string): Promise<Thread> {
		const thread = this.threads.find((t) => t.id === threadId);
		if (!thread) throw new Error('Thread not found');
		return { ...thread, title: `Generated title for ${threadId}` };
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		const needle = query.trim().toLowerCase();
		if (!needle) return [];
		return this.threads
			.filter((t) => t.title.toLowerCase().includes(needle))
			.slice(offset, offset + limit)
			.map((t) => ({ id: t.id, title: t.title, rank: 1 }));
	}

	async searchMessages(
		_query: string,
		_limit: number,
		_offset: number,
	): Promise<MessageSearchResult[]> {
		return [];
	}

	async *sendMessage(
		_threadId: string,
		_text: string,
		_options?: SendMessageOptions,
	): AsyncIterable<ChatStreamEvent> {
		for (const chunk of this.streamChunks) {
			if (this.streamDelay > 0) {
				await new Promise((r) => setTimeout(r, this.streamDelay));
			}
			yield chunk;
		}
	}
}
