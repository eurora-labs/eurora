import { provideAll } from '@eurora/shared/context';
// import { type AppSettings } from '$lib/bindings/bindings.js';
import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';

// export function initDependencies(args: { appSettings: AppSettings }) {
// 	const { appSettings } = args.appSettings;
export function initDependencies() {
	return provideAll([[TAURPC_SERVICE, createTauRPCProxy()]]);
}
