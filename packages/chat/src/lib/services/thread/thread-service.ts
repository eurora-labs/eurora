import { InjectionToken } from '@eurora/shared/context';
import {
	type ListThreadsRequest,
	type DeleteThreadRequest,
} from '@eurora/shared/proto/thread_service_pb.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { BaseMessageWithSibling } from '@eurora/shared/proto/agent_chain_pb.js';

export interface IThreadService {
	listThreads(request: ListThreadsRequest): Promise<Thread[]>;
	getMessages(threadId: string, limit: number, offset: number): Promise<BaseMessageWithSibling[]>;
	switchBranch(
		threadId: string,
		messageId: string,
		direction: number,
	): Promise<BaseMessageWithSibling[]>;
	deleteThread(request: DeleteThreadRequest): Promise<void>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
