import { InjectionToken } from '@eurora/shared/context';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { Thread } from '$lib/models/thread.model.js';

export type BranchDirection = -1 | 0 | 1;

export interface IThreadService {
	listThreads(limit: number, offset: number): Promise<Thread[]>;
	getMessages(threadId: string, limit: number, offset: number): Promise<MessageNode[]>;
	switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]>;
	deleteThread(threadId: string): Promise<void>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
