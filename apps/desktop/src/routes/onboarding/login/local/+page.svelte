<script lang="ts">
	import { goto } from '$app/navigation';
	import { type APISettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let endpoint = $state('http://localhost:39051');
	let connecting = $state(false);

	onMount(async () => {
		const settings = await taurpc.settings.get_api_settings();
		if (settings.endpoint) {
			endpoint = settings.endpoint;
		}
	});

	async function connect() {
		connecting = true;
		try {
			await taurpc.system.check_grpc_server_connection(endpoint);

			const settings: APISettings = {
				endpoint,
				provider: null,
			};
			await taurpc.settings.set_api_settings(settings);
			goto('/onboarding/login/local/auth');
		} catch (error) {
			toast.error(`Could not connect to ${endpoint}`);
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
			placeholder="http://localhost:39051"
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
