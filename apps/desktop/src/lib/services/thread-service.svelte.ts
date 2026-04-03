import { type IThreadService } from '@eurora/chat/services/thread/thread-service';
import { InjectionToken } from '@eurora/shared/context';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { Thread } from '@eurora/chat/models/thread.model';
import type { MessageNode } from '@eurora/chat/models/messages/index';
import { toMessageNodes } from '$lib/services/converters/message-converter.js';

export class ThreadService implements IThreadService {
	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		return (await this.taurpc.thread.list(limit, offset)).map(
			(thread) =>
				({
					id: thread.id,
					title: thread.title,
					createdAt: thread.created_at,
					updatedAt: thread.updated_at,
				}) as Thread,
		);
	}

	async getMessages(threadId: string, limit: number, offset: number): Promise<MessageNode[]> {
		const raw = await this.taurpc.thread.get_messages(threadId, limit, offset);
		return toMessageNodes(raw);
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: number,
	): Promise<MessageNode[]> {
		const raw = await this.taurpc.thread.switch_branch(threadId, messageId, direction);
		return toMessageNodes(raw);
	}

	async deleteThread(threadId: string): Promise<void> {
		await this.taurpc.thread.delete(threadId);
	}
}

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
