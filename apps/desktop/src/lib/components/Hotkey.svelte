<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Badge } from '@eurora/ui/components/badge/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import type { Hotkey } from '$lib/bindings/bindings.js';

	let taurpc = createTauRPCProxy();
	let settingHotkey = $state(false);
	let currentHotkey = $state<Hotkey>({ modifiers: ['Ctrl'], key: 'Space' });
	let recordedHotkey = $state<Hotkey | null>(null);
	let isRecording = $state(false);
	let recordingTimeout: NodeJS.Timeout | null = null;

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
				currentHotkey.key.toLowerCase(),
				currentHotkey.modifiers.map((modifier) => modifier.toLowerCase()),
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
				['Escape', 'Enter', 'Tab', 'Space'].includes(event.key) ||
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
			currentHotkey = recordedHotkey;
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

<div class="flex flex-col justify-center items-center gap-6">
	<div class="text-center">
		<h2 class="text-lg font-semibold mb-2">Current hotkey:</h2>
		<Badge variant="outline" class="text-lg">{hotkeyToString(currentHotkey)}</Badge>
	</div>

	{#if isRecording && recordedHotkey}
		<div class="text-center">
			<p class="text-sm mb-2">Recording...</p>
			<Badge variant="outline" class="text-lg">{hotkeyToString(recordedHotkey)}</Badge>
			<p class="text-xs text-gray-500 dark:text-gray-400 mt-2">Release keys to confirm</p>
		</div>
	{/if}

	<div class="flex gap-3">
		<Button disabled={settingHotkey} onclick={setHotkey} variant="secondary" class="min-w-32">
			{#if settingHotkey}
				<Loader2Icon class="animate-spin mr-2" size={16} />
				{isRecording ? 'Press keys...' : 'Starting...'}
			{:else}
				Set hotkey
			{/if}
		</Button>

		{#if isRecording}
			<Button onclick={cancelRecording} variant="outline">Cancel</Button>
		{/if}
	</div>

	{#if isRecording}
		<div class="text-center max-w-md">
			<p class="text-sm text-gray-600 dark:text-gray-400">
				Press a key combination (e.g., Ctrl+Shift+A).
				<br />
				Make sure to include at least one modifier key.
			</p>
		</div>
	{/if}
</div>
