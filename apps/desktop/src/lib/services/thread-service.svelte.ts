import { toChatStreamEvent, toMessageNodes } from '$lib/services/converters/message-converter.js';
import { InjectionToken } from '@eurora/shared/context';
import type { ChatStreamResponse, MessageTreeNodeView, Query } from '$lib/bindings/bindings.js';
import type { TaurpcService } from '$lib/bindings/taurpcService.js';
import type { ContentBlock } from '@eurora/chat/models/content-blocks/index';
import type { Message, MessageNode } from '@eurora/chat/models/messages/index';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	IThreadService,
	MessageTreeResult,
} from '@eurora/chat/services/thread/thread-service';

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

	async getMessageTree(
		threadId: string,
		startLevel: number,
		endLevel: number,
		parentNodeIds: string[],
	): Promise<MessageTreeResult> {
		const raw = await this.taurpc.thread.get_message_tree(
			threadId,
			startLevel,
			endLevel,
			parentNodeIds,
		);
		return {
			roots: buildMessageTree(raw.nodes),
			hasMore: raw.has_more,
		};
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
		signal?: AbortSignal,
	): AsyncIterable<ChatStreamEvent> {
		const query: Query = {
			text,
			assets: [],
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

		function onEvent(response: ChatStreamResponse) {
			buffer.push(toChatStreamEvent(response));
			notify();
		}

		const onAbort = () => {
			this.taurpc.chat.cancel_query(threadId).catch(() => {});
			notify();
		};
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
		}
	}
}

function buildMessageTree(flatNodes: MessageTreeNodeView[]): MessageNode[] {
	const nodeMap = new Map<string, MessageNode>();

	for (const raw of flatNodes) {
		nodeMap.set(raw.id, {
			parentId: raw.parent_message_id,
			message: toTreeMessage(raw),
			children: [],
			siblingIndex: raw.sibling_index,
			depth: raw.level,
		});
	}

	const roots: MessageNode[] = [];
	for (const raw of flatNodes) {
		const node = nodeMap.get(raw.id)!;
		const parent = raw.parent_message_id ? nodeMap.get(raw.parent_message_id) : undefined;
		if (parent) {
			parent.children.push(node);
		} else {
			roots.push(node);
		}
	}

	return roots;
}

function toTreeMessage(raw: MessageTreeNodeView): Message {
	const content: ContentBlock[] = raw.content
		? [
				{
					type: 'text',
					id: null,
					text: raw.content,
					annotations: [],
					index: null,
					extras: null,
				},
			]
		: [];

	switch (raw.message_type) {
		case 'human':
			return {
				type: 'human',
				content,
				id: raw.id,
				name: null,
				additionalKwargs: null,
				responseMetadata: null,
			};
		case 'ai':
			return {
				type: 'ai',
				content,
				id: raw.id,
				name: null,
				toolCalls: [],
				invalidToolCalls: [],
				usageMetadata: null,
				additionalKwargs: null,
				responseMetadata: null,
			};
		default:
			return {
				type: 'chat',
				role: raw.message_type,
				content,
				id: raw.id,
				name: null,
				additionalKwargs: null,
				responseMetadata: null,
			};
	}
}

export const THREAD_SERVICE = new InjectionToken<ThreadService>('ThreadService');
