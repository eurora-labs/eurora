import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
import { THREAD_SERVICE, ThreadService } from '$lib/services/thread-service.svelte.js';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const taurpc = createTauRPCProxy();
	return provideAll([
		[TAURPC_SERVICE, taurpc],
		[THREAD_SERVICE, new ThreadService(taurpc)],
	]);
}
