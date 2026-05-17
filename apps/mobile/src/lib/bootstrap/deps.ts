import { ThreadService } from '$lib/services/thread-service.svelte.js';
import { USER_SERVICE, UserService } from '$lib/services/user-service.svelte.js';
import { ChatService, CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
import { THREAD_SERVICE } from '@eurora/chat/services/thread/thread-service';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const threadClient = new ThreadService();

	provideAll([
		[THREAD_SERVICE, threadClient],
		[USER_SERVICE, new UserService()],
		[CHAT_SERVICE, new ChatService(threadClient)],
	]);
}
