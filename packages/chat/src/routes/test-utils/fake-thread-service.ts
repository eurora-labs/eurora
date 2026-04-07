import type { IThreadService, BranchDirection } from '$lib/services/thread/thread-service.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { ChatStreamEvent } from '$lib/models/streaming.js';

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

export class FakeThreadService implements IThreadService {
	threads: Thread[] = [];
	messagesByThread = new Map<string, MessageNode[]>();

	deleteDelay = 0;
	shouldFailDelete = false;

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
		_threadId: string,
		_messageId: string,
		_direction: BranchDirection,
	): Promise<MessageNode[]> {
		return [];
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

	async *sendMessage(
		_threadId: string,
		_text: string,
		_parentMessageId?: string | null,
		_signal?: AbortSignal,
		_assetIds?: string[],
	): AsyncIterable<ChatStreamEvent> {
		// no-op for sidebar tests
	}
}
