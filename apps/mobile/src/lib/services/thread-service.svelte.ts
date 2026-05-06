import { fromChatServerMessage } from '@eurora/chat/models/streaming';
import type { Query } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { MessageSearchResult, ThreadSearchResult } from '@eurora/chat/models/search.model';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	BranchDirection,
	IThreadService,
	SendMessageOptions,
} from '@eurora/chat/services/thread/thread-service';
import type {
	ChatServerMessage,
	MessageNode,
	Thread as WireThread,
} from '@eurora/shared/bindings/thread';

export class ThreadService implements IThreadService {
	private readonly taurpc: TaurpcService;

	constructor(taurpc: TaurpcService) {
		this.taurpc = taurpc;
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		return (await this.taurpc.thread.list(limit, offset)) as unknown as Thread[];
	}

	async getMessages(
		threadId: string,
		limit: number,
		offset: number,
		allVariants: boolean,
	): Promise<MessageNode[]> {
		return (await this.taurpc.thread.get_messages(
			threadId,
			limit,
			offset,
			allVariants,
		)) as unknown as MessageNode[];
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]> {
		return (await this.taurpc.thread.switch_branch(
			threadId,
			messageId,
			direction,
		)) as unknown as MessageNode[];
	}

	async deleteThread(threadId: string): Promise<void> {
		await this.taurpc.thread.delete(threadId);
	}

	async createThread(): Promise<Thread> {
		return (await this.taurpc.thread.create()) as unknown as WireThread;
	}

	async generateTitle(threadId: string): Promise<Thread> {
		return (await this.taurpc.thread.generate_title(threadId)) as unknown as WireThread;
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		return (await this.taurpc.thread.search_threads(
			query,
			limit,
			offset,
		)) as unknown as ThreadSearchResult[];
	}

	async searchMessages(
		query: string,
		limit: number,
		offset: number,
	): Promise<MessageSearchResult[]> {
		return (await this.taurpc.thread.search_messages(
			query,
			limit,
			offset,
		)) as unknown as MessageSearchResult[];
	}

	async *sendMessage(
		threadId: string,
		text: string,
		options: SendMessageOptions = {},
	): AsyncIterable<ChatStreamEvent> {
		const { parentMessageId, signal, assetChips } = options;
		const query: Query = {
			text,
			assets: assetChips?.map((c) => c.id) ?? [],
			parent_message_id: parentMessageId ?? null,
		};
		const buffer: ChatStreamEvent[] = [];
		let resolve: ((value: void) => void) | null = null;
		let finished = false;
		let error: unknown = null;

		function notify() {
			resolve?.();
			resolve = null;
		}

		// Until taurpc regenerates its local bindings (which happens at app
		// runtime, not at `cargo check`), the channel callback's type still
		// reflects the pre-typing wire shape, so we narrow via `unknown`.
		function onEvent(response: unknown) {
			try {
				buffer.push(fromChatServerMessage(response as ChatServerMessage));
			} catch (e) {
				error = e;
				finished = true;
			}
			notify();
		}

		function onAbort() {
			notify();
		}
		signal?.addEventListener('abort', onAbort, { once: true });

		this.taurpc.chat.send_query(threadId, onEvent, query).then(
			() => {
				finished = true;
				notify();
			},
			(e: unknown) => {
				error = e;
				finished = true;
				notify();
			},
		);

		try {
			while (true) {
				while (buffer.length > 0) {
					if (signal?.aborted) return;
					yield buffer.shift()!;
				}

				if (finished) break;
				if (signal?.aborted) return;

				await new Promise<void>((r) => {
					resolve = r;
				});
			}

			while (buffer.length > 0) {
				yield buffer.shift()!;
			}

			if (error) throw error;
		} finally {
			signal?.removeEventListener('abort', onAbort);
			if (!finished) {
				this.taurpc.chat.cancel_query(threadId).catch(() => {});
			}
		}
	}
}
