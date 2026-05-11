<script lang="ts">
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import { platform } from '@tauri-apps/plugin-os';
	import { useDebounce, useInterval } from 'runed';
	import { onMount } from 'svelte';

	const STARTUP_DELAY_MS = 1_000;
	const POLL_INTERVAL_MS = 2_000;
	const MAX_POLL_ATTEMPTS = 30;

	let dialogOpen = $state(false);

	async function checkPermission(): Promise<boolean> {
		try {
			return await commands.systemCheckAccessibilityPermission();
		} catch (error) {
			console.error('Failed to check accessibility permission:', error);
			return true; // Assume granted on error to avoid blocking
		}
	}

	const permissionPoll = useInterval(POLL_INTERVAL_MS, {
		immediate: false,
		callback: async (count) => {
			const granted = await checkPermission();
			if (granted) {
				dialogOpen = false;
				permissionPoll.pause();
				return;
			}
			if (count >= MAX_POLL_ATTEMPTS) {
				permissionPoll.pause();
			}
		},
	});

	// `checking` is whatever the poll says — single source of truth.
	const checking = $derived(permissionPoll.isActive);

	async function requestPermission() {
		permissionPoll.reset();
		try {
			await commands.systemRequestAccessibilityPermission();
		} catch (error) {
			console.error('Failed to request accessibility permission:', error);
		}
		permissionPoll.resume();
	}

	function dismiss() {
		permissionPoll.pause();
		dialogOpen = false;
	}

	const startupCheck = useDebounce(async () => {
		const granted = await checkPermission();
		if (!granted) dialogOpen = true;
	}, STARTUP_DELAY_MS);

	onMount(() => {
		// Only relevant on macOS
		if (platform() !== 'macos') return;
		startupCheck();
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
