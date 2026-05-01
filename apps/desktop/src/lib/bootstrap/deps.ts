import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
import { ThreadService } from '$lib/services/thread-service.svelte.js';
import { TIMELINE_SERVICE, TimelineService } from '$lib/services/timeline-service.svelte.js';
import { USER_SERVICE, UserService } from '$lib/services/user-service.svelte.js';
import { ChatService, CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
import { THREAD_SERVICE } from '@eurora/chat/services/thread/thread-service';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const taurpc = createTauRPCProxy();
	const threadClient = new ThreadService(taurpc);
	return provideAll([
		[TAURPC_SERVICE, taurpc],
		[THREAD_SERVICE, threadClient],
		[USER_SERVICE, new UserService(taurpc)],
		[CHAT_SERVICE, new ChatService(threadClient)],
		[TIMELINE_SERVICE, new TimelineService(taurpc)],
	]);
}
