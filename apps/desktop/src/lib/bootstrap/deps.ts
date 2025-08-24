import { provideAll } from '@eurora/shared/context';
// import { type AppSettings } from '$lib/bindings/bindings.js';
import { HotkeyService, HOTKEY_SERVICE } from '$lib/hotkey/hotkeyService';

// export function initDependencies(args: { appSettings: AppSettings }) {
// 	const { appSettings } = args.appSettings;
export function initDependencies() {
	const hotkeyService = new HotkeyService();

	return provideAll([[HOTKEY_SERVICE, hotkeyService]]);
}
