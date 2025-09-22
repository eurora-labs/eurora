<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import CheckCircleIcon from '@lucide/svelte/icons/check-circle';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import { default as HotkeyComponent } from '$lib/components/Hotkey.svelte';
	import type { Hotkey, LauncherSettings } from '$lib/bindings/bindings.js';
	import { goto } from '$app/navigation';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';

	import { onMount } from 'svelte';

	let taurpc = createTauRPCProxy();

	let launcherSettings = $state<LauncherSettings | undefined>(undefined);
	let hotkey = $state<Hotkey | undefined>(undefined);
	let launcherOpened = $state(false);
	let waitingForHotkey = $state(true);
	let countdown = $state(5);
	let buttonDisabled = $state(true);
	let countdownInterval: ReturnType<typeof setInterval> | undefined;

	onMount(() => {
		taurpc.settings.get_launcher_settings().then((settings) => {
			launcherSettings = settings;
			hotkey = settings.hotkey;
			waitingForHotkey = false;
		});

		// Listen for launcher_opened event
		let unlistenLauncherOpened: (() => void) | undefined;
		taurpc.window.launcher_opened
			.on(async () => {
				launcherOpened = true;
				// Skip countdown if launcher is opened
				if (countdownInterval) {
					clearInterval(countdownInterval);
					countdownInterval = undefined;
				}
				countdown = 0;
				buttonDisabled = false;
			})
			.then((unsub) => {
				unlistenLauncherOpened = unsub;
			});
		// Start countdown timer
		countdownInterval = setInterval(() => {
			countdown--;
			if (countdown <= 0) {
				buttonDisabled = false;
				if (countdownInterval) {
					clearInterval(countdownInterval);
				}
			}
		}, 1000);

		return () => {
			unlistenLauncherOpened?.();
			if (countdownInterval) {
				clearInterval(countdownInterval);
			}
		};
	});

	async function onHotkeyChange(key: Hotkey) {
		if (!launcherSettings) {
			console.warn('Launcher settings not loaded yet; skipping hotkey persist');
			return;
		}
		await taurpc.settings.set_launcher_settings({ ...launcherSettings, hotkey: key });
		hotkey = key;
		launcherOpened = false; // Reset success state when hotkey changes
	}

	function handleBack() {
		goto('/onboarding/hotkey');
	}

	function handleContinue() {
		goto('/');
	}
</script>

<div class="w-full h-full p-6 flex flex-col justify-between">
	<div class="flex-1 flex flex-col justify-center">
		{#if waitingForHotkey}
			<div class="flex items-center justify-center py-8">
				<p class="text-muted-foreground">Loading your hotkey...</p>
			</div>
		{:else if hotkey}
			{#if !launcherOpened}
				<!-- Instructions -->
				<div class="text-center mb-4">
					<h2 class="text-2xl font-semibold mb-4">
						Launch Eurora by pressing <span class="font-medium"
							>{hotkey.modifiers.join(' + ')} + {hotkey.key}</span
						>
					</h2>
				</div>

				<div class="flex flex-col items-center justify-center py-8 gap-4">
					<p class="text-muted-foreground">Or change the hotkey here</p>
					<p>
						<HotkeyComponent variant="default" {hotkey} {onHotkeyChange} />
					</p>
				</div>
			{/if}

			<!-- Success State -->
			{#if launcherOpened}
				<div class="flex items-center justify-center py-6 mb-6">
					<div class="flex items-center gap-3 text-green-600">
						<CheckCircleIcon class="w-6 h-6" />
						<h2 class="font-medium text-2xl">
							Great! Everything is ready to start using Eurora!
						</h2>
					</div>
				</div>
			{/if}
		{/if}
	</div>

	<!-- Navigation -->
	<div class="flex justify-between">
		<Button variant="ghost" onclick={handleBack}>Back</Button>

		<Button onclick={handleContinue} disabled={buttonDisabled}>
			{#if buttonDisabled}
				Skip in {countdown}s
			{:else if launcherOpened}
				Continue
				<ArrowRightIcon class="w-4 h-4 ml-2" />
			{:else}
				Skip
			{/if}
		</Button>
	</div>
</div>
