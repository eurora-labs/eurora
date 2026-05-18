import { CommandError, unwrap, type CommandResult } from '$lib/bindings/result.js';
import { commands, type StreamError } from '$lib/bindings/specta.bindings.js';
import { Channel } from '@tauri-apps/api/core';

import type { MessageNode } from '@eurora/chat/models/messages/index';
import type { MessageSearchResult, ThreadSearchResult } from '@eurora/chat/models/search.model';
import type { ChatServerMessage } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	BranchDirection,
	ChatContext,
	IThreadService,
} from '@eurora/chat/services/thread/thread-service';
import type { ChatSendRequest } from '@eurora/shared/bindings/thread';

export class ThreadService implements IThreadService {
	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		return unwrap(await commands.threadList(limit, offset));
	}

	async listThreadsForActivity(activityId: string): Promise<Thread[]> {
		return unwrap(await commands.threadListByActivity(activityId, 100, 0));
	}

	async getMessages(threadId: string, limit: number, offset: number): Promise<MessageNode[]> {
		return unwrap(await commands.threadGetMessages(threadId, limit, offset));
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]> {
		return unwrap(await commands.threadSwitchBranch(threadId, messageId, direction));
	}

	async deleteThread(threadId: string): Promise<void> {
		unwrap(await commands.threadDelete(threadId));
	}

	async createThread(): Promise<Thread> {
		return unwrap(await commands.threadCreate());
	}

	async generateTitle(threadId: string): Promise<Thread> {
		return unwrap(await commands.threadGenerateTitle(threadId));
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		return unwrap(await commands.threadSearchThreads(query, limit, offset));
	}

	async searchMessages(
		query: string,
		limit: number,
		offset: number,
	): Promise<MessageSearchResult[]> {
		return unwrap(await commands.threadSearchMessages(query, limit, offset));
	}

	async collectContext(_threadId: string): Promise<ChatContext> {
		// Mobile has no desktop activity timeline — chat turns carry only the
		// user's text.
		return { contentBlocks: [], assetChips: [] };
	}

	sendMessage(
		threadId: string,
		request: ChatSendRequest,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		return this.#streamChat(
			threadId,
			(channel) => commands.chatSendQuery(threadId, channel, request),
			signal,
		);
	}

	regenerateAi(
		threadId: string,
		aiMessageId: string,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		return this.#streamChat(
			threadId,
			(channel) => commands.chatRegenerate(threadId, aiMessageId, channel),
			signal,
		);
	}

	async *#streamChat(
		threadId: string,
		open: (channel: Channel<ChatServerMessage>) => Promise<CommandResult<null, StreamError>>,
		signal?: AbortSignal,
	): AsyncIterable<ChatServerMessage> {
		const buffer: ChatServerMessage[] = [];
		let resolve: ((value: void) => void) | null = null;
		let finished = false;
		let error: unknown = null;

		function notify() {
			resolve?.();
			resolve = null;
		}

		const channel = new Channel<ChatServerMessage>();
		channel.onmessage = (response) => {
			buffer.push(response);
			notify();
		};

		function onAbort() {
			notify();
		}
		signal?.addEventListener('abort', onAbort, { once: true });

		open(channel).then(
			(result) => {
				// `Cancelled` is the user-driven AbortSignal path racing the
				// server — surface as a clean stream end, not an error.
				if (result.status === 'error' && result.error.type !== 'Cancelled') {
					error = new CommandError(result.error);
				}
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

			while (buffer.length > 0) yield buffer.shift()!;

			if (error) throw error;
		} finally {
			signal?.removeEventListener('abort', onAbort);
			if (!finished) {
				commands
					.chatCancelQuery(threadId)
					.then(unwrap)
					.catch(() => {});
			}
		}
	}
}
