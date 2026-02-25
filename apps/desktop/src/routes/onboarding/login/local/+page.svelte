<script lang="ts">
	import { goto } from '$app/navigation';
	import { type APISettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let endpoint = $state('http://localhost:39051');

	onMount(async () => {
		const settings = await taurpc.settings.get_api_settings();
		if (settings.endpoint) {
			endpoint = settings.endpoint;
		}
	});

	async function save() {
		try {
			const settings: APISettings = {
				endpoint,
				provider: null,
			};
			await taurpc.settings.set_api_settings(settings);
			toast.success('Endpoint saved');
			goto('/');
		} catch (error) {
			toast.error(`Failed to save: ${error}`);
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
		<Input id="endpoint" placeholder="http://localhost:39051" bind:value={endpoint} />
		<p class="text-xs text-muted-foreground">The address of your self-hosted Eurora backend.</p>
	</div>

	<div class="flex justify-between">
		<Button variant="outline" onclick={() => goto('/onboarding/login')}>Back</Button>
		<Button onclick={save}>Connect</Button>
	</div>
</div>
