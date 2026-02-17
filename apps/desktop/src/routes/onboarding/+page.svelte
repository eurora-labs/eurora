<script lang="ts">
	import { goto } from '$app/navigation';
	import { type TelemetrySettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { onMount } from 'svelte';

	let taurpc = inject(TAURPC_SERVICE);

	let errorReporting = $state(false);
	let usageMetrics = $state(false);
	let nonAnonymousUsageMetrics = $state(false);
	let telemetrySettings: TelemetrySettings | undefined = $state();

	onMount(() => {
		taurpc.settings
			.get_telemetry_settings()
			.then((settings) => {
				if (settings.considered) {
					goToLogin();
				}

				telemetrySettings = settings;
				errorReporting = settings.anonymousErrors;
				usageMetrics = settings.anonymousMetrics;
				nonAnonymousUsageMetrics = settings.nonAnonymousMetrics;
			})
			.catch((error) => {
				console.error('Failed to fetch telemetry settings:', error);
			});
	});

	async function updateSettings() {
		if (!telemetrySettings) return;

		telemetrySettings.considered = true;
		telemetrySettings.anonymousErrors = errorReporting;
		telemetrySettings.anonymousMetrics = usageMetrics;
		telemetrySettings.nonAnonymousMetrics = nonAnonymousUsageMetrics;

		try {
			telemetrySettings = await taurpc.settings.set_telemetry_settings(telemetrySettings);
		} catch (error) {
			console.error('Failed to update telemetry settings:', error);
		}

		goToLogin();
	}

	function goToLogin() {
		goto('/onboarding/login');
	}
</script>

<div class="relative flex h-full w-full flex-col">
	<div class="flex flex-col justify-center items-start h-full w-full px-8">
		<article class="pb-4">
			<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Welcome to Eurora!</h1>
			<p class="pb-2">
				Eurora uses these metrics strictly to help us improve the app. We do not collect any
				personal information unless you yourself choose to provide it.
			</p>
			<p>
				I ask you to please keep these settings enabled. Eurora is self-funded and these
				metrics are essential for us to improve the app and stay competitive with
				billionaire-funded apps.
			</p>
		</article>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Error reporting</Item.Title>
				<Item.Description>Report crashes and errors.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={errorReporting} />
			</Item.Actions>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Usage metrics</Item.Title>
				<Item.Description>Provide anonymous usage metrics.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={usageMetrics} />
			</Item.Actions>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Non-anonymous usage metrics</Item.Title>
				<Item.Description>Share of detailed usage metrics.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={nonAnonymousUsageMetrics} />
			</Item.Actions>
		</Item.Root>
		<div class="flex justify-end">
			<Button
				variant="default"
				onclick={() => {
					updateSettings();
				}}
				>Continue
				<ChevronRight />
			</Button>
		</div>
	</div>

	<div class="pb-16"></div>
</div>
