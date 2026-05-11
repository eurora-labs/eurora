import { Channel, invoke } from '@tauri-apps/api/core';

import { AppAuthError } from './types';
import type { AuthEvent } from './types';

export type AuthEventHandler = (event: AuthEvent) => void;

/**
 * Detaches a previously-registered {@link AuthEventHandler}.
 *
 * The native plugin tracks at most one event channel per session; calling
 * {@link onAuthEvent} again replaces the active subscription. Calling the
 * returned `Unsubscribe` only stops the local handler from firing — it does
 * not signal the native side.
 */
export type Unsubscribe = () => void;

/**
 * Subscribe to the plugin's diagnostic event stream.
 *
 * Each call registers a fresh `Channel<AuthEvent>` with the native side via
 * `subscribe_events`; the prior subscription, if any, is replaced. Returns
 * an `Unsubscribe` that detaches the local handler.
 */
export async function onAuthEvent(handler: AuthEventHandler): Promise<Unsubscribe> {
    const channel = new Channel<AuthEvent>();
    let active = true;
    channel.onmessage = (event) => {
        if (active) {
            handler(event);
        }
    };

    try {
        await invoke('plugin:appauth|subscribe_events', { channel });
    } catch (error) {
        active = false;
        throw AppAuthError.from(error);
    }

    return () => {
        active = false;
    };
}
