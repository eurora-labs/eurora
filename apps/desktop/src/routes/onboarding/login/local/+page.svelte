<!-- TODO: This needs to be remade completely -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		commands,
		type APISettings,
		type ConnectionMode,
	} from '$lib/bindings/specta.bindings.js';
	import { unwrap } from '$lib/bindings/result.js';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let endpoint = $state('http://localhost:3000');
	let connecting = $state(false);

	function modeFor(url: string): ConnectionMode {
		// localhost:3000 is the canonical Local mode URL — store as Local so
		// the persisted config survives a future port-default change.
		if (url === 'http://localhost:3000' || url === 'http://127.0.0.1:3000') {
			return { kind: 'custom', url: 'http://localhost:3000' };
		}
		return { kind: 'custom', url };
	}

	onMount(async () => {
		const settings = await commands.settingsGetApi();
		if (settings.mode.kind === 'custom') {
			endpoint = settings.mode.url;
		} else if (settings.mode.kind === 'default') {
			endpoint = 'http://localhost:3000';
		}
	});

	async function connect() {
		connecting = true;
		try {
			// `test_backend_url` hits /llm/info which fails fast if the URL
			// doesn't speak Eurora's protocol — better than the old TCP-only
			// reachability check, which would happily greenlight a random
			// HTTP server on the same port.
			unwrap(await commands.systemTestBackendUrl(endpoint));

			const settings: APISettings = { mode: modeFor(endpoint) };
			unwrap(await commands.settingsSetApi(settings));
			goto('/onboarding/login/local/auth');
		} catch (error) {
			toast.error(`Could not connect to ${endpoint}. Error: ${error}`);
		} finally {
			connecting = false;
		}
	}
</script>

<div class="flex flex-col justify-center h-full px-8 gap-6">
	<div>
		<h1 class="text-3xl font-bold mb-2">Local Setup</h1>
		<p class="text-sm text-muted-foreground">
			Run Eurora with your own backend and models. Follow the
			<Button
				variant="link"
				class="inline-flex h-auto p-0 text-sm"
				onclick={() => open('https://www.eurora-labs.com/docs/self-hosting')}
			>
				self-hosting guide
				<ExternalLink class="size-3" />
			</Button>
			to get started.
		</p>
	</div>

	<div class="flex flex-col gap-2">
		<Label for="endpoint" class="text-sm font-medium">API Endpoint</Label>
		<Input
			id="endpoint"
			placeholder="http://localhost:3000"
			bind:value={endpoint}
			disabled={connecting}
		/>
		<p class="text-xs text-muted-foreground">The address of your self-hosted Eurora backend.</p>
	</div>

	<div class="flex justify-between">
		<Button variant="outline" onclick={() => goto('/onboarding/login')} disabled={connecting}
			>Back</Button
		>
		<Button onclick={connect} disabled={connecting}>
			{#if connecting}
				<Spinner class="size-4" />
				Connecting...
			{:else}
				Connect
			{/if}
		</Button>
	</div>
</div>
