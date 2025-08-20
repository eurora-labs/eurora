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

	let taurpc = createTauRPCProxy();
	let settingHotkey = $state(false);
	let recordedHotkey = $state<Hotkey | null>(null);
	let isRecording = $state(false);
	let recordingTimeout: NodeJS.Timeout | null = null;

	let { hotkey, onHotkeyChange, variant = 'secondary' }: HotkeyProps = $props();

	// Key mapping for better display
	const keyDisplayMap: Record<string, string> = {
		Control: 'Ctrl',
		Meta: 'Cmd',
		Alt: 'Alt',
		Shift: 'Shift',
		' ': 'Space',
		ArrowUp: '↑',
		ArrowDown: '↓',
		ArrowLeft: '←',
		ArrowRight: '→',
		Escape: 'Esc',
		Enter: 'Enter',
		Tab: 'Tab',
		Backspace: 'Backspace',
		Delete: 'Del',
	};

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

	function getKeyDisplay(key: string): string {
		return keyDisplayMap[key] || key.toUpperCase();
	}

	function hotkeyToString(hotkey: Hotkey): string {
		return [...hotkey.modifiers, hotkey.key].join(' + ');
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (!isRecording) return;

		event.preventDefault();
		event.stopPropagation();

		// Handle Escape key to cancel recording
		if (event.key === 'Escape') {
			cancelRecording();
			return;
		}

		const modifiers: string[] = [];

		// Add modifiers in consistent order
		if (event.ctrlKey || event.metaKey) {
			modifiers.push(event.ctrlKey ? 'Ctrl' : 'Cmd');
		}
		if (event.altKey) {
			modifiers.push('Alt');
		}
		if (event.shiftKey) {
			modifiers.push('Shift');
		}

		// Add the main key (if it's not a modifier)
		if (!['Control', 'Meta', 'Alt', 'Shift'].includes(event.key)) {
			const key = getKeyDisplay(event.key);

			// Only update if we have at least one modifier + main key, or special keys
			if (
				modifiers.length >= 1 ||
				['Enter', 'Tab', 'Space'].includes(event.key) ||
				/^F\d+$/.test(event.key)
			) {
				recordedHotkey = { modifiers, key };

				// Clear existing timeout
				if (recordingTimeout) {
					clearTimeout(recordingTimeout);
				}

				// Set timeout to finalize recording
				recordingTimeout = setTimeout(() => {
					finalizeRecording();
				}, 1000);
			}
		}
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

	// Add global event listener when recording
	$effect(() => {
		if (isRecording) {
			document.addEventListener('keydown', handleKeyDown, true);

			return () => {
				document.removeEventListener('keydown', handleKeyDown, true);
			};
		}
	});

	// Cleanup effect
	$effect(() => {
		return cleanup;
	});
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
