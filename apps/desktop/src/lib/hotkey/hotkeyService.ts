import { InjectionToken } from '@eurora/shared/context';
import type { Hotkey } from '$lib/bindings/bindings.js';
import { toString as event2String, details as eventDetails } from 'keyboard-event-to-string';

export const HOTKEY_SERVICE = new InjectionToken<HotkeyService>('HotkeyService');

export class HotkeyService {
	public interpretHotkey(event: KeyboardEvent): Hotkey | null {
		const details = eventDetails(event);
		if (!details.hasKey) return null;
		const keys = event2String(event);

		const modifiers = keys.split(' + ');
		const key = modifiers.pop();

		if (!key) throw new Error('Malformed hotkey string');

		return { modifiers, key };
	}
}
