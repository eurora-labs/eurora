import { provideAll } from '@eurora/shared/context';
// import { type AppSettings } from '$lib/bindings/bindings.js';
import { HotkeyService, HOTKEY_SERVICE } from '$lib/hotkey/hotkeyService';
import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { TAURPC_SERVICE } from '$lib/bindings/taurpcService';

// export function initDependencies(args: { appSettings: AppSettings }) {
// 	const { appSettings } = args.appSettings;
export function initDependencies() {
	const hotkeyService = new HotkeyService();

	return provideAll([
		[TAURPC_SERVICE, createTauRPCProxy()],
		[HOTKEY_SERVICE, hotkeyService],
	]);
}
