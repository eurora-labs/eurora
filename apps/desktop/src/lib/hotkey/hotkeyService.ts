import { InjectionToken } from '@eurora/shared/context';
import type { Hotkey } from '$lib/bindings/bindings.js';
import { toString as event2String, details as eventDetails } from 'keyboard-event-to-string';

export const HotkeyServiceToken = new InjectionToken<HotkeyService>('HotkeyService');

export class HotkeyService {
	public interpretHotkey(event: KeyboardEvent): Hotkey | null {
		const details = eventDetails(event);
		if (!details.hasKey) return null;
		const keys = event2String(event);

		// The aggregated keys should contain at least one regular key
		// if (
		// 	!keys.includes('Key') &&
		// 	!keys.includes('Digit') &&
		// 	!keys.includes('Numpad') &&
		// 	!keys.includes('Space')
		// )
		// 	return null;

		console.log(keys);

		const modifiers = keys.split(' + ');
		let key = modifiers.pop();

		if (!key) throw new Error('Malformed hotkey string');
		console.log(modifiers, key);

		return { modifiers, key };
	}
}
