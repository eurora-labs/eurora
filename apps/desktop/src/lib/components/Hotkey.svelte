<script lang="ts" module>
	import { type ButtonVariant } from '@eurora/ui/components/button/index';
	export interface HotkeyProps {
		hotkey: Hotkey;
		onHotkeyChange?: (hotkey: Hotkey) => void;
		variant?: ButtonVariant;
	}
</script>

<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import type { Hotkey } from '$lib/bindings/bindings.js';
	import { HOTKEY_SERVICE } from '$lib/hotkey/hotkeyService.js';
	import { inject } from '@eurora/shared/context';
	import { onMount } from 'svelte';

	let hotkeyService = inject(HOTKEY_SERVICE);

	let taurpc = createTauRPCProxy();
	let settingHotkey = $state(false);
	let recordedHotkey = $state<Hotkey | null>(null);
	let isRecording = $state(false);
	let recordingTimeout: NodeJS.Timeout | null = null;

	let { hotkey, onHotkeyChange, variant = 'secondary' }: HotkeyProps = $props();

	export async function saveHotkey() {
		try {
			await taurpc.user.set_launcher_hotkey(
				hotkey.key.toLowerCase(),
				hotkey.modifiers.map((modifier: string) => modifier.toLowerCase()),
			);
		} catch (error) {
			console.error('Error setting hotkey:', error);
		}
	}

	function hotkeyToString(hotkey: Hotkey): string {
		return [...hotkey.modifiers, hotkey.key].join(' + ');
	}

	function handleKeyDown(event: KeyboardEvent) {
		console.log(event);
		if (!isRecording) return;
		window.focus();

		event.preventDefault();
		event.stopPropagation();

		// Handle Escape key to cancel recording
		if (event.key === 'Escape') {
			cancelRecording();
			return;
		}

		const interpretedHotkey = hotkeyService.interpretHotkey(event);
		if (!interpretedHotkey) return;
		recordedHotkey = interpretedHotkey;
		hotkey = interpretedHotkey;

		isRecording = false;

		// Clear existing timeout
		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
		}

		// Set timeout to finalize recording
		recordingTimeout = setTimeout(() => {
			finalizeRecording();
		}, 1000);
	}

	async function finalizeRecording() {
		if (recordedHotkey) {
			hotkey = recordedHotkey;
			onHotkeyChange?.(recordedHotkey);
		}

		isRecording = false;
		settingHotkey = false;
		recordedHotkey = null;

		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
			recordingTimeout = null;
		}
	}

	function cancelRecording() {
		isRecording = false;
		settingHotkey = false;
		recordedHotkey = null;

		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
			recordingTimeout = null;
		}
	}

	onMount(() => {
		document.body.addEventListener('keydown', handleKeyDown);

		return () => {
			document.body.removeEventListener('keydown', handleKeyDown);
			cleanup();
		};
	});

	async function setHotkey() {
		settingHotkey = true;
		isRecording = true;
		recordedHotkey = null;

		// Focus the window to ensure we capture key events
		window.focus();
	}

	// Cleanup on component destroy
	function cleanup() {
		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
		}
	}
</script>

{#if hotkey}
	<Button disabled={settingHotkey} onclick={setHotkey} {variant} class="min-w-32">
		{#if settingHotkey}
			<Loader2Icon class="animate-spin mr-2" size={16} />
			{#if isRecording}
				{#if recordedHotkey}
					{hotkeyToString(recordedHotkey)}
				{:else}
					Recording keys...
				{/if}
			{:else}
				Starting...
			{/if}
		{:else}
			{hotkeyToString(hotkey)}
		{/if}
	</Button>
{/if}
