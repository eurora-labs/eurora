import { InjectionToken } from '@eurora/shared/context';
import type { Hotkey } from '$lib/bindings/bindings.js';
import { toString as event2String } from 'keyboard-event-to-string';

export const HotkeyServiceToken = new InjectionToken<HotkeyService>('HotkeyService');

export class HotkeyService {
	public interpretHotkey(event: KeyboardEvent): Hotkey | null {
		const keys = event2String(event);
		// The aggregated keys should contain at least one regular key
		if (!keys.includes('Key') && !keys.includes('Space')) return null;

		const modifiers = keys.split(' + ');
		let key = modifiers.pop();
		if (key !== 'Space') {
			key = key?.slice(3);
		}

		if (!key) throw new Error('Malformed hotkey string');

		return { modifiers, key };
	}
}
