import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	return provideAll([[TAURPC_SERVICE, createTauRPCProxy()]]);
}
