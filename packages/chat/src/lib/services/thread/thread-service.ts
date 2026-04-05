import { InjectionToken } from '@eurora/shared/context';
import type { MessageNode } from '$lib/models/messages/index.js';
import type { ChatStreamEvent } from '$lib/models/streaming.js';
import type { Thread } from '$lib/models/thread.model.js';
import type { MessageTreeResponse } from '$lib/models/tree.js';

export type BranchDirection = -1 | 0 | 1;

export interface IThreadService {
	listThreads(limit: number, offset: number): Promise<Thread[]>;
	getMessages(threadId: string, limit: number, offset: number): Promise<MessageNode[]>;
	getMessageTree(
		threadId: string,
		startLevel: number,
		endLevel: number,
		parentNodeIds: string[],
	): Promise<MessageTreeResponse>;
	switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]>;
	deleteThread(threadId: string): Promise<void>;
	createThread(): Promise<Thread>;
	sendMessage(
		threadId: string,
		text: string,
		parentMessageId?: string | null,
		signal?: AbortSignal,
	): AsyncIterable<ChatStreamEvent>;
}

export const THREAD_SERVICE = new InjectionToken<IThreadService>('ThreadService');
