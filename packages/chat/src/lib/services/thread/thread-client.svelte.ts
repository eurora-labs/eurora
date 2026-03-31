import type { ProtoBaseMessage } from '@eurora/shared/proto/agent_chain_pb.js';
import type { Thread } from '@eurora/shared/proto/thread_service_pb.js';
import type { Readable } from 'svelte/store';

export interface ThreadClient {
	newThread$: Readable<Thread>;
	titleChanged$: Readable<Thread>;
	currentThreadChanged$: Readable<Thread>;

	list(limit: number, offset: number): Promise<Thread[]>;
	getMessages(threadId: string, limit: number, offset: number): Promise<ProtoBaseMessage[]>;
	delete(threadId: string): Promise<void>;
}
