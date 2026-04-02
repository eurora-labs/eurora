import type { Thread } from '$lib/models/thread.model.js';
import {
	type MessageTreeNode,
	ProtoThreadService,
	type ListThreadsRequest,
	type DeleteThreadRequest,
} from '@eurora/shared/proto/thread_service_pb.js';
import type { ProtoBaseMessage } from '@eurora/shared/proto/agent_chain_pb.js';
import { InjectionToken } from '@eurora/shared/context';

export interface IThreadService {
	listThreads(request: ListThreadsRequest): Promise<Thread[]>;
	loadMoreMessages(): Promise<ProtoBaseMessage[]>;
	deleteThread(request: DeleteThreadRequest): Promise<void>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
