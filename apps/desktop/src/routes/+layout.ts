import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { redirect } from '@sveltejs/kit';

export const prerender = true;
export const ssr = false;

export async function load({ url }) {
	if (url.pathname.startsWith('/onboarding')) {
		return {};
	}
	try {
		const taurpc = createTauRPCProxy();
		if (await taurpc.system.needs_telemetry_consent()) {
			redirect(307, '/onboarding');
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
