import { commands, events } from '$lib/bindings/specta.bindings.js';
import { redirect } from '@sveltejs/kit';

export const prerender = true;
export const ssr = false;

export async function load({ url }) {
	if (url.pathname.startsWith('/onboarding')) {
		return {};
	}
	// The ask / answer overlay windows are spawned by the Tauri host
	// directly via `tauri::WebviewUrl::App("ask"|"answer"…)`. They run
	// in their own webviews with their own minimal layout shell and
	// must not be gated on the main window's telemetry-consent prompt
	// — the gate event is delivered only to a webview that issued
	// `frontendReady`, and these overlays deliberately don't.
	if (url.pathname.startsWith('/ask') || url.pathname.startsWith('/answer')) {
		return {};
	}
	try {
		// Rust pushes the gate state via the `ConsentGate` event. We attach
		// a one-shot listener, hit `frontendReady` to make the backend emit,
		// and wait for the event. The frontend never compares
		// `consent_version` itself — the rule lives entirely in Rust.
		const required = await awaitInitialConsentGate();
		if (required) {
			redirect(307, '/onboarding/telemetry');
		}
	} catch (error) {
		// Let SvelteKit propagate its own redirect error.
		if ((error as { status?: number })?.status === 307) {
			throw error;
		}
		// Failing closed (showing onboarding) is the safe default — but
		// if the IPC bridge isn't reachable (e.g. tests, prerender) we
		// don't want to wedge the entire UI on a redirect loop. Log and
		// fall through.
		console.error('Failed to gate telemetry onboarding:', error);
	}
	return {};
}

/**
 * Wait for Rust to emit the first {@link events.consentGate} after we
 * trigger `frontendReady`. Listener registration is awaited before the
 * IPC fires so the event cannot outrun the subscription — Tauri events
 * don't replay, so a late listener would deadlock the gate.
 */
async function awaitInitialConsentGate(): Promise<boolean> {
	let resolveGate!: (required: boolean) => void;
	const gate = new Promise<boolean>((resolve) => {
		resolveGate = resolve;
	});

	// `once` returns a promise for the unlisten fn that resolves after
	// the listener is registered with the Tauri runtime. The unlisten
	// fires automatically when the event arrives, so we don't keep it.
	await events.consentGate.once((event) => {
		resolveGate(event.payload.required);
	});

	await commands.frontendReady();
	return await gate;
}
