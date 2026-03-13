import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
import { MESSAGE_SERVICE, MessageService } from '$lib/services/message-service.svelte.js';
import { THREAD_SERVICE, ThreadService } from '$lib/services/thread-service.svelte.js';
import { USER_SERVICE, UserService } from '$lib/services/user-service.svelte.js';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const taurpc = createTauRPCProxy();
	return provideAll([
		[TAURPC_SERVICE, taurpc],
		[THREAD_SERVICE, new ThreadService(taurpc)],
		[MESSAGE_SERVICE, new MessageService(taurpc)],
		[USER_SERVICE, new UserService(taurpc)],
	]);
}
