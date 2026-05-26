import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { MessageSearchResult, ThreadSearchResult } from '$lib/models/search.model.js';
import type { ChatServerMessage } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type {
	BranchDirection,
	ChatContext,
	IThreadService,
} from '$lib/services/thread/thread-service.js';

let nextId = 1;

function makeThread(overrides?: Partial<Thread>): Thread {
	const id = `thread-${nextId++}`;
	const now = new Date().toISOString();
	return {
		id,
		user_id: '',
		title: `Thread ${id}`,
		created_at: now,
		updated_at: now,
		...overrides,
	} as Thread;
}

export function makeMessageNode(
	id: string,
	type: 'human' | 'ai',
	text: string,
	overrides?: Partial<MessageNode>,
): MessageNode {
	const content: ContentBlock[] =
		text.length > 0
			? [{ type: 'text', id: null, text, annotations: null, index: null, extras: null }]
			: [];

	const base: MessageNode = {
		parent_id: null,
		message:
			type === 'human'
				? {
						type: 'human',
						content,
						id,
						name: null,
						additional_kwargs: {},
						response_metadata: {},
					}
				: {
						type: 'ai',
						content,
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
	return { ...base, ...overrides };
}

export function makeReasoningNode(id: string, reasoning: string, text: string): MessageNode {
	const content: ContentBlock[] = [
		{ type: 'reasoning', id: null, reasoning, index: null, extras: null },
	];
	if (text) {
		content.push({
			type: 'text',
			id: null,
			text,
			annotations: null,
			index: null,
			extras: null,
		});
	}
	return {
		parent_id: null,
		message: {
			type: 'ai',
			content,
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

export class FakeThreadService implements IThreadService {
	threads: Thread[] = [];
	messagesByThread = new Map<string, MessageNode[]>();
	branchResults = new Map<string, MessageNode[]>();
	threadsByActivity = new Map<string, Thread[]>();

	deleteDelay = 0;
	shouldFailDelete = false;

	streamFrames: ChatServerMessage[] = [];
	streamDelay = 0;

	context: ChatContext = { assetChips: [] };

	seed(count: number): void {
		this.threads = Array.from({ length: count }, (_, i) =>
			makeThread({ title: `Chat ${i + 1}` }),
		);
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		return this.threads.slice(offset, offset + limit);
	}

	async listThreadsForActivity(activityId: string): Promise<Thread[]> {
		return this.threadsByActivity.get(activityId) ?? [];
	}

	async getMessages(threadId: string, _limit: number, _offset: number): Promise<MessageNode[]> {
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

	async generateTitle(threadId: string): Promise<Thread> {
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
			.map((t) => ({ id: t.id, title: t.title, rank: 1, updated_at: t.updated_at }));
	}

	async searchMessages(
		_query: string,
		_limit: number,
		_offset: number,
	): Promise<MessageSearchResult[]> {
		return [];
	}

	async collectContext(_threadId: string): Promise<ChatContext> {
		return this.context;
	}

	async *sendMessage(
		_threadId: string,
		_request: unknown,
		_signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		for (const frame of this.streamFrames) {
			if (this.streamDelay > 0) {
				await new Promise((r) => setTimeout(r, this.streamDelay));
			}
			yield frame;
		}
	}

	async *regenerateAi(
		_threadId: string,
		_aiMessageId: string,
		_signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		for (const frame of this.streamFrames) {
			if (this.streamDelay > 0) {
				await new Promise((r) => setTimeout(r, this.streamDelay));
			}
			yield frame;
		}
	}
}
