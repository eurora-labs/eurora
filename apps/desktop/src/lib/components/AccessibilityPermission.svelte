<script lang="ts">
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	const taurpcService = inject(TAURPC_SERVICE);

	let dialogOpen = $state(false);
	let checking = $state(false);
	let cancelled = false;

	async function checkPermission(): Promise<boolean> {
		try {
			return await taurpcService.system.check_accessibility_permission();
		} catch (error) {
			console.error('Failed to check accessibility permission:', error);
			return true; // Assume granted on error to avoid blocking
		}
	}

	async function requestPermission() {
		checking = true;
		cancelled = false;
		try {
			await taurpcService.system.request_accessibility_permission();
		} catch (error) {
			console.error('Failed to request accessibility permission:', error);
		}
		await pollForPermission();
		checking = false;
	}

	async function pollForPermission() {
		for (let i = 0; i < 30; i++) {
			if (cancelled) return;
			await new Promise((resolve) => setTimeout(resolve, 2000));
			if (cancelled) return;
			const granted = await checkPermission();
			if (granted) {
				dialogOpen = false;
				return;
			}
		}
	}

	function dismiss() {
		cancelled = true;
		dialogOpen = false;
	}

	onMount(() => {
		// Only relevant on macOS
		if (platform() !== 'macos') return;

		const timeout = setTimeout(async () => {
			const granted = await checkPermission();
			if (!granted) {
				dialogOpen = true;
			}
		}, 1000);

		return () => clearTimeout(timeout);
	});
</script>

<Dialog.Root bind:open={dialogOpen}>
	<Dialog.Content class="sm:max-w-[480px]" onInteractOutside={(e) => e.preventDefault()}>
		<div class="flex flex-col items-center gap-4 pt-2">
			<div class="flex items-center justify-center rounded-full bg-amber-500/10 p-3">
				<ShieldCheckIcon class="size-8 text-amber-500" />
			</div>

			<Dialog.Header class="text-center">
				<Dialog.Title class="text-center text-lg">
					Accessibility Permission Required
				</Dialog.Title>
				<Dialog.Description class="text-center text-sm text-muted-foreground">
					Eurora needs accessibility access to track which application you're working in.
					This enables context-aware assistance while you code.
				</Dialog.Description>
			</Dialog.Header>

			<div class="w-full rounded-lg border border-border bg-muted/50 p-4 text-sm">
				<p class="mb-2 font-medium">How to enable:</p>
				<ol class="list-inside list-decimal space-y-1 text-muted-foreground">
					<li>
						Click <span class="font-medium text-foreground">Open Settings</span> below
					</li>
					<li>
						Find <span class="font-medium text-foreground">Eurora</span> in the list
					</li>
					<li>Toggle the switch to enable access</li>
				</ol>
			</div>
		</div>

		<Dialog.Footer class="mt-2 flex-col gap-2 sm:flex-col">
			<Button onclick={requestPermission} disabled={checking} class="w-full">
				{#if checking}
					Waiting for permission...
				{:else}
					Open Settings
				{/if}
			</Button>
			<Button variant="ghost" onclick={dismiss} class="w-full">Remind me later</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
