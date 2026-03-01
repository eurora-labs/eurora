<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { open } from '@tauri-apps/plugin-shell';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount, onDestroy } from 'svelte';

	const isMacos = platform() === 'macos';
	const taurpc = inject(TAURPC_SERVICE);

	let downloaded = $state(false);
	let connected = $state(false);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	onMount(async () => {
		try {
			const count = await taurpc.system.get_browser_connection_count();
			if (count > 0) {
				goto('/');
			}
		} catch (_) {}
	});

	async function downloadBrowserExtension() {
		const url = await taurpc.onboarding.get_browser_extension_download_url();
		await open(url);
		downloaded = true;

		intervalId = setInterval(async () => {
			try {
				const count = await taurpc.system.get_browser_connection_count();
				if (count > 0) {
					connected = true;
					if (intervalId) clearInterval(intervalId);
				}
			} catch (err) {
				console.error('Failed to check browser connections:', err);
			}
		}, 2000);
	}

	onDestroy(() => {
		if (intervalId) clearInterval(intervalId);
	});
</script>

{#if !downloaded}
	<div class="relative flex h-full w-full flex-col px-8">
		<div class="flex flex-col justify-center items-start h-full w-full gap-6">
			<div>
				<h1 class="text-3xl font-bold mb-2">Browser Extension</h1>
				<p class="text-sm text-muted-foreground">
					Eurora uses a browser extension to understand your browsing context, enabling deeper
					integration and more relevant assistance as you work.
				</p>
			</div>

			<Button onclick={downloadBrowserExtension}>
				Download Extension
				<ExternalLink class="size-3" />
			</Button>

			{#if isMacos}
				<Alert>
					<AlertDescription>
						<p class="font-medium text-foreground">Using Safari?</p>
						<p class="mt-1">After downloading, enable the extension manually:</p>
						<ol class="mt-2 list-decimal list-inside space-y-1">
							<li>Open <span class="font-medium">Safari Settings</span> (⌘,)</li>
							<li>Click the <span class="font-medium">Extensions</span> tab</li>
							<li>Find <span class="font-medium">Eurora</span> and check the box to enable it</li>
							<li>Click <span class="font-medium">"Turn On"</span> to confirm</li>
						</ol>
					</AlertDescription>
				</Alert>
			{/if}
		</div>

		<div class="mb-8">
			<Button variant="outline" onclick={() => goto('/onboarding')}>Back</Button>
		</div>
	</div>
{:else}
	<div class="relative flex h-full w-full flex-col px-8">
		<div class="flex flex-col justify-center items-center h-full w-full gap-6">
			<div class="flex flex-row items-center gap-4">
				{#if !connected}
					<Spinner class="w-8 h-8" />
					<h1 class="text-4xl font-bold drop-shadow-lg">
						Waiting for extension to connect...
					</h1>
				{:else}
					<h1 class="text-4xl font-bold drop-shadow-lg">Extension connected!</h1>
				{/if}
			</div>
			<p class="text-sm text-muted-foreground max-w-lg text-center">
				Eurora uses a browser extension to understand your browsing context, enabling deeper
				integration and more relevant assistance as you work.
			</p>
		</div>
		<div class="flex justify-between mb-8">
			<Button
				variant="outline"
				onclick={() => {
					if (intervalId) {
						clearInterval(intervalId);
						intervalId = null;
					}
					downloaded = false;
				}}>Cancel</Button>
			{#if connected}
				<Button onclick={() => goto('/')}>Continue</Button>
			{/if}
		</div>
	</div>
{/if}
