import { toChatStreamEvent, toMessageNodes } from '$lib/services/converters/message-converter.js';
import type { ChatStreamResponse, ContextChip, Query } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { AssetChip, MessageNode } from '@eurora/chat/models/messages/index';
import type { MessageSearchResult, ThreadSearchResult } from '@eurora/chat/models/search.model';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	IThreadService,
	SendMessageOptions,
} from '@eurora/chat/services/thread/thread-service';

type ExtendedQuery = Query & { preserved_asset_chips: ContextChip[] | null };

function toContextChip(chip: AssetChip): ContextChip {
	return { id: chip.id, name: chip.name, icon: chip.icon, domain: chip.domain };
}

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

	async getMessages(
		threadId: string,
		limit: number,
		offset: number,
		allVariants: boolean,
	): Promise<MessageNode[]> {
		const raw = await this.taurpc.thread.get_messages(threadId, limit, offset, allVariants);
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
			id: raw.id!,
			title: raw.title,
			createdAt: raw.created_at,
			updatedAt: raw.updated_at,
		};
	}

	async generateTitle(threadId: string, content: string): Promise<Thread> {
		const raw = await this.taurpc.thread.generate_title(threadId, content);
		return {
			id: raw.id!,
			title: raw.title,
			createdAt: raw.created_at,
			updatedAt: raw.updated_at,
		};
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		const raw = await this.taurpc.thread.search_threads(query, limit, offset);
		return raw.map((r) => ({ id: r.id, title: r.title, rank: r.rank }));
	}

	async searchMessages(
		query: string,
		limit: number,
		offset: number,
	): Promise<MessageSearchResult[]> {
		const raw = await this.taurpc.thread.search_messages(query, limit, offset);
		return raw.map((r) => ({
			id: r.id,
			threadId: r.thread_id,
			messageType: r.message_type,
			snippet: r.snippet,
			rank: r.rank,
		}));
	}

	async *sendMessage(
		threadId: string,
		text: string,
		options: SendMessageOptions = {},
	): AsyncIterable<ChatStreamEvent> {
		const { parentMessageId, signal, assetChips, preservedAssetChips } = options;
		const query: ExtendedQuery = {
			text,
			assets: assetChips?.map((c) => c.id) ?? [],
			parent_message_id: parentMessageId ?? null,
			preserved_asset_chips: preservedAssetChips?.map(toContextChip) ?? null,
		};
		const buffer: ChatStreamEvent[] = [];
		let resolve: ((value: void) => void) | null = null;
		let finished = false;
		let error: unknown = null;

		function notify() {
			resolve?.();
			resolve = null;
		}

		function onEvent(response: ChatStreamResponse) {
			buffer.push(toChatStreamEvent(response));
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
