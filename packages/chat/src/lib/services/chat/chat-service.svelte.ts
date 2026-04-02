import { createClient, type Client } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { type ConfigService } from '@eurora/shared/config/config-service';
import { InjectionToken } from '@eurora/shared/context';
import {
	type Thread,
	type MessageTreeNode,
	ProtoThreadService,
	type ListThreadsRequest,
} from '@eurora/shared/proto/thread_service_pb.js';
import type { ProtoBaseMessage } from '@eurora/shared/proto/agent_chain_pb.js';

const PAGE_SIZE = 20;
const MAX_LOAD_RETRIES = 3;

export class ThreadMessages {
	thread: Thread;
	messages: ProtoBaseMessage[] = $state([]);
	treeNodes: MessageTreeNode[] = $state([]);
	loading = $state(false);
	hasMore = $state(true);
	offset = 0;
	streaming = $state(false);

	treeLoadedEndLevel = 0;
	treeLoading = $state(false);
	treeHasMore = $state(false);
	treeLoadId = 0;
	treeInitialLoaded = false;

	constructor(thread: Thread) {
		this.thread = thread;
	}
}

export class ChatService {
	newThread: Thread | undefined = $state();
	titleChanged: Thread | undefined = $state();
	currentThreadChanged: Thread | undefined = $state();

	threads: ThreadMessages[] = $state([]);
	loading = $state(true);
	loadingMore = $state(false);
	hasMore = $state(true);
	activeThreadId: string | null = $state(null);

	private offset = 0;
	private loadRetries = 0;
	private _client: Client<typeof ProtoThreadService> | null = null;
	private readonly config: ConfigService;
	private readonly unlisteners: ((() => void) | Promise<() => void>)[] = [];

	private get client(): Client<typeof ProtoThreadService> {
		if (!this._client) {
			this._client = createClient(
				ProtoThreadService,
				createGrpcWebTransport({
					baseUrl: this.config.grpcApiUrl,
					useBinaryFormat: true,
				}),
			);
		}
		return this._client;
	}

	constructor(config: ConfigService) {
		this.config = config;
	}

	async loadThreads(limit: number, offset: number) {
		try {
			this.threads = (
				await this.client.listThreads({ limit, offset } as ListThreadsRequest)
			).threads.map((thread) => new ThreadMessages(thread));
			this.offset = this.threads.length;
			this.hasMore = this.threads.length === PAGE_SIZE;
		} catch (error) {
			console.error('Failed to load threads:', error);
		} finally {
			this.loading = false;
		}
	}

	updateThread(thread: Thread) {
		this.threads = this.threads.map((t) =>
			t.thread.id === thread.id ? { ...t, title: thread.title } : t,
		);
	}

	destroy() {
		for (const p of this.unlisteners) {
			if (p instanceof Promise) {
				p.then((unlisten) => unlisten());
			} else {
				p();
			}
		}
		this.unlisteners.length = 0;
		this.threads = [];
		this.offset = 0;
		this.loadRetries = 0;
		this.hasMore = true;
		this.loading = true;
		this.loadingMore = false;
		this.activeThreadId = null;
	}
}

export const CHAT_SERVICE = new InjectionToken<ChatService>('ChatService');
