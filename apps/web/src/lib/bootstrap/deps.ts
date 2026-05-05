import { AUTH_SERVICE, AuthService } from '$lib/services/auth-service.svelte.js';
import { DOWNLOAD_SERVICE, DownloadService } from '$lib/services/download-service.js';
import {
	SUBSCRIPTION_SERVICE,
	SubscriptionService,
} from '$lib/services/subscription-service.svelte.js';
import { ThreadService } from '$lib/services/thread-service.svelte.js';
import { CHAT_SERVICE, ChatService } from '@eurora/chat/services/chat/chat-service.svelte';
import { THREAD_SERVICE } from '@eurora/chat/services/thread/thread-service';
import { CONFIG_SERVICE, ConfigService } from '@eurora/shared/config/config-service';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const config = new ConfigService(
		import.meta.env.VITE_GRPC_API_URL,
		import.meta.env.VITE_REST_API_URL ?? import.meta.env.VITE_GRPC_API_URL,
	);
	const auth = new AuthService(config);
	const threadClient = new ThreadService(config, auth);

	provideAll([
		[CONFIG_SERVICE, config],
		[AUTH_SERVICE, auth],
		[DOWNLOAD_SERVICE, new DownloadService(config)],
		[SUBSCRIPTION_SERVICE, new SubscriptionService(auth, config)],
		[THREAD_SERVICE, threadClient],
		[CHAT_SERVICE, new ChatService(threadClient)],
	]);
}
