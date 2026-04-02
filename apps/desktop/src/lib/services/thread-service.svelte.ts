import { type IThreadService } from '@eurora/chat/services/thread/thread-service';
import { InjectionToken } from '@eurora/shared/context';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { Thread } from '@eurora/chat/models/thread.model';
import type { BaseMessageWithSibling } from '@eurora/shared/proto/agent_chain_pb.js';
import type {
	DeleteThreadRequest,
	ListThreadsRequest,
} from '@eurora/shared/proto/thread_service_pb.js';

export class ThreadService implements IThreadService {
	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async listThreads(request: ListThreadsRequest): Promise<Thread[]> {
		return (await this.taurpc.thread.list(request.limit, request.offset)).map(
			(thread) =>
				({
					id: thread.id,
					title: thread.title,
					createdAt: thread.created_at,
					updatedAt: thread.updated_at,
				}) as Thread,
		);
	}

	async getMessages(
		threadId: string,
		limit: number,
		offset: number,
	): Promise<BaseMessageWithSibling[]> {
		return this.taurpc.thread.get_messages(threadId, limit, offset) as unknown as Promise<
			BaseMessageWithSibling[]
		>;
	}

	async deleteThread(request: DeleteThreadRequest): Promise<void> {
		await this.taurpc.thread.delete(request.threadId);
	}
}

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
