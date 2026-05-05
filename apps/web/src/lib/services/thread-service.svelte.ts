import {
	toChatStreamEvent,
	toMessageNodes,
	toThread,
} from '$lib/services/converters/message-converter.js';
import { createAuthedTransport } from '$lib/services/grpc-transport.js';
import { create, type MessageInitShape } from '@bufbuild/protobuf';
import { createClient, type Client } from '@connectrpc/connect';
import { ProtoContentBlockSchema } from '@eurora/shared/proto/agent_chain_pb.js';
import {
	ChatStreamRequestSchema,
	CreateThreadRequestSchema,
	DeleteThreadRequestSchema,
	GenerateThreadTitleRequestSchema,
	GetMessagesRequestSchema,
	ListThreadsRequestSchema,
	ProtoThreadService,
	SearchMessagesRequestSchema,
	SearchThreadsRequestSchema,
	SwitchBranchRequestSchema,
} from '@eurora/shared/proto/thread_service_pb.js';
import type { AuthService } from '$lib/services/auth-service.svelte.js';
import type { AssetChip, MessageNode } from '@eurora/chat/models/messages/index';
import type { MessageSearchResult, ThreadSearchResult } from '@eurora/chat/models/search.model';
import type { ChatStreamEvent } from '@eurora/chat/models/streaming';
import type { Thread } from '@eurora/chat/models/thread.model';
import type {
	BranchDirection,
	IThreadService,
	SendMessageOptions,
} from '@eurora/chat/services/thread/thread-service';
import type { ConfigService } from '@eurora/shared/config/config-service';

export class ThreadService implements IThreadService {
	readonly #config: ConfigService;
	readonly #auth: AuthService;
	#client: Client<typeof ProtoThreadService> | null = null;

	constructor(config: ConfigService, auth: AuthService) {
		this.#config = config;
		this.#auth = auth;
	}

	async listThreads(limit: number, offset: number): Promise<Thread[]> {
		const resp = await this.#grpc.listThreads(
			create(ListThreadsRequestSchema, { limit, offset }),
		);
		return resp.threads.map((t) => toThread(t));
	}

	async getMessages(
		threadId: string,
		limit: number,
		offset: number,
		allVariants: boolean,
	): Promise<MessageNode[]> {
		const resp = await this.#grpc.getMessages(
			create(GetMessagesRequestSchema, { threadId, limit, offset, allVariants }),
		);
		return toMessageNodes(resp.messages);
	}

	async switchBranch(
		threadId: string,
		messageId: string,
		direction: BranchDirection,
	): Promise<MessageNode[]> {
		const resp = await this.#grpc.switchBranch(
			create(SwitchBranchRequestSchema, { threadId, messageId, direction }),
		);
		return toMessageNodes(resp.messages);
	}

	async deleteThread(threadId: string): Promise<void> {
		await this.#grpc.deleteThread(create(DeleteThreadRequestSchema, { threadId }));
	}

	async createThread(): Promise<Thread> {
		const resp = await this.#grpc.createThread(
			create(CreateThreadRequestSchema, { title: '' }),
		);
		return toThread(resp.thread);
	}

	async generateTitle(threadId: string, content: string): Promise<Thread> {
		const resp = await this.#grpc.generateThreadTitle(
			create(GenerateThreadTitleRequestSchema, { threadId, content }),
		);
		return toThread(resp.thread);
	}

	async searchThreads(
		query: string,
		limit: number,
		offset: number,
	): Promise<ThreadSearchResult[]> {
		const resp = await this.#grpc.searchThreads(
			create(SearchThreadsRequestSchema, { query, limit, offset }),
		);
		return resp.results.map((r) => ({ id: r.id, title: r.title, rank: r.rank }));
	}

	async searchMessages(
		query: string,
		limit: number,
		offset: number,
	): Promise<MessageSearchResult[]> {
		const resp = await this.#grpc.searchMessages(
			create(SearchMessagesRequestSchema, { query, limit, offset }),
		);
		return resp.results.map((r) => ({
			id: r.id,
			threadId: r.threadId,
			messageType: r.messageType,
			snippet: r.snippet,
			rank: r.rank,
		}));
	}

	async *sendMessage(
		threadId: string,
		text: string,
		options: SendMessageOptions = {},
	): AsyncIterable<ChatStreamEvent> {
		const { parentMessageId, signal, assetChips } = options;
		const request = create(ChatStreamRequestSchema, {
			threadId,
			contentBlocks: [textContentBlock(text)],
			parentMessageId: parentMessageId ?? undefined,
			assetChipsJson: serializeAssetChips(assetChips),
		});

		const stream = this.#grpc.chatStream(request, { signal });
		for await (const response of stream) {
			yield toChatStreamEvent(response);
		}
	}

	get #grpc(): Client<typeof ProtoThreadService> {
		this.#client ??= createClient(
			ProtoThreadService,
			createAuthedTransport(this.#config, this.#auth),
		);
		return this.#client;
	}
}

function textContentBlock(text: string): MessageInitShape<typeof ProtoContentBlockSchema> {
	return {
		block: { case: 'text', value: { text, annotations: [] } },
	};
}

function serializeAssetChips(chips: AssetChip[] | undefined): string | undefined {
	if (!chips || chips.length === 0) return undefined;
	return JSON.stringify(chips);
}
