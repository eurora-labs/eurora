import { toChatStreamEvent, toMessageNodes } from '$lib/services/converters/message-converter.js';
import { InjectionToken } from '@eurora/shared/context';
import type { ChatStreamResponse, Query } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { MessageNode } from '@eurora/chat/models/messages/index';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type { IThreadService } from '@eurora/chat/services/thread/thread-service';

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

	async createThread(): Promise<Thread> {
		const raw = await this.taurpc.thread.create();
		return {
			id: raw.id,
			title: raw.title,
			createdAt: raw.created_at,
			updatedAt: raw.updated_at,
		};
	}

	async *sendMessage(
		threadId: string,
		text: string,
		parentMessageId?: string | null,
	): AsyncIterable<ChatStreamEvent> {
		const query: Query = {
			text,
			assets: [],
			parent_message_id: parentMessageId ?? null,
		};

		const buffer: ChatStreamEvent[] = [];
		let resolve: (() => void) | null = null;

		function onEvent(response: ChatStreamResponse) {
			buffer.push(toChatStreamEvent(response));
			resolve?.();
		}

		const done = this.taurpc.chat.send_query(threadId, onEvent, query).then(() => true);

		while (true) {
			if (buffer.length > 0) {
				yield buffer.shift()!;
				continue;
			}

			const finished = await Promise.race([
				done,
				new Promise<false>((r) => {
					resolve = () => r(false);
				}),
			]);

			if (finished) {
				while (buffer.length > 0) {
					yield buffer.shift()!;
				}
				break;
			}
		}
	}
}

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
