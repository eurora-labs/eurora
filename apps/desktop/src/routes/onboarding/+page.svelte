<script lang="ts">
	import { goto } from '$app/navigation';
	import { commands, type TelemetrySettings } from '$lib/bindings/specta.bindings.js';
	import { unwrap } from '$lib/bindings/result.js';
	import { TELEMETRY_SERVICE } from '$lib/services/telemetry-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { onMount } from 'svelte';

	let telemetry = inject(TELEMETRY_SERVICE);

	let errorReporting = $state(true);
	let usageMetrics = $state(true);
	let nonAnonymousUsageMetrics = $state(false);
	let telemetrySettings: TelemetrySettings | undefined = $state();
	let saving = $state(false);

	onMount(() => {
		commands
			.settingsGetTelemetry()
			.then((settings) => {
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
		if (!telemetrySettings || saving) return;

		saving = true;
		try {
			telemetrySettings = unwrap(
				await commands.settingsSetTelemetry({
					...telemetrySettings,
					anonymousErrors: errorReporting,
					anonymousMetrics: usageMetrics,
					nonAnonymousMetrics: nonAnonymousUsageMetrics,
				}),
			);
			await telemetry.refresh();
		} catch (error) {
			console.error('Failed to update telemetry settings:', error);
			saving = false;
			return;
		}

		goToLogin();
	}

	function goToLogin() {
		goto('/onboarding/login');
	}
</script>

<div class="flex flex-col justify-center h-full px-8 gap-6">
	<div>
		<h1 class="text-3xl font-bold mb-2">Welcome to Eurora!</h1>
		<p class="text-sm text-muted-foreground mb-2">
			Eurora uses these metrics strictly to help us improve the app. We do not collect any
			personal information unless you yourself choose to provide it.
		</p>
		<p class="text-sm text-muted-foreground">
			I ask you to please keep these settings enabled. Eurora is self-funded and these metrics
			are essential for us to improve the app and stay competitive with billionaire-funded
			apps.
		</p>
	</div>

	<Item.Group class="gap-2">
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
				<Switch bind:checked={nonAnonymousUsageMetrics} disabled={!usageMetrics} />
			</Item.Actions>
		</Item.Root>
	</Item.Group>

	<div class="flex justify-end">
		<Button
			variant="default"
			disabled={saving}
			onclick={() => {
				updateSettings();
			}}
			>Continue
			<ChevronRight />
		</Button>
	</div>
</div>
