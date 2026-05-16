<script lang="ts">
	import { unwrap } from '$lib/bindings/result.js';
	import {
		commands,
		type TelemetryConsent,
		type TelemetryLocal,
	} from '$lib/bindings/specta.bindings.js';
	import { TELEMETRY_SERVICE } from '$lib/services/telemetry-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const telemetry = inject(TELEMETRY_SERVICE);

	let consent = $state<TelemetryConsent | null>(null);
	let local = $state<TelemetryLocal | null>(null);
	let saving = $state(false);
	let rotating = $state(false);

	onMount(async () => {
		try {
			[consent, local] = await Promise.all([
				commands.settingsGetTelemetryConsent(),
				commands.settingsGetLocalTelemetry(),
			]);
		} catch (error) {
			console.error('Failed to load telemetry settings:', error);
			toast.error('Could not load telemetry settings');
		}
	});

	/**
	 * Persist a fresh consent decision through the dedicated consent IPC.
	 * The backend stamps the consent version monotonically, lazily
	 * allocates a `distinct_id`, and reapplies the native Sentry guard —
	 * none of which `settings_set_desktop` does, since that procedure is
	 * scoped to non-consent desktop state.
	 */
	async function persist(nextConsent: TelemetryConsent) {
		saving = true;
		try {
			const updated = unwrap(await commands.settingsRecordTelemetryConsent(nextConsent));
			consent = updated;
			await telemetry.refresh();
		} catch (error) {
			console.error('Failed to save telemetry settings:', error);
			toast.error('Could not save telemetry settings');
		} finally {
			saving = false;
		}
	}

	async function toggle(field: keyof TelemetryConsent, value: boolean) {
		if (!consent) return;
		await persist({ ...consent, [field]: value });
	}

	async function rotateId() {
		if (!local) return;
		rotating = true;
		try {
			await telemetry.rotateDistinctId();
			local = await commands.settingsGetLocalTelemetry();
			toast.success('Telemetry id rotated');
		} catch (error) {
			console.error('Failed to rotate telemetry id:', error);
			toast.error('Could not rotate telemetry id');
		} finally {
			rotating = false;
		}
	}

	async function copyId() {
		if (!local?.distinctId) return;
		await navigator.clipboard.writeText(local.distinctId);
		toast.success('Telemetry id copied');
	}
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">Telemetry</h1>
		<p class="text-sm text-muted-foreground">
			Control what usage data is shared with the Eurora team.
		</p>
	</div>

	{#if consent && local}
		<section class="flex flex-col gap-4">
			<h2 class="text-sm font-medium text-muted-foreground">Data collection</h2>
			<Separator />
			<div class="flex items-center justify-between">
				<div>
					<Label for="anon-errors" class="text-sm">Anonymous error reports</Label>
					<p class="text-xs text-muted-foreground">
						Send crash and error reports so we can fix bugs.
					</p>
				</div>
				<Switch
					id="anon-errors"
					checked={consent.anonymousErrors ?? false}
					disabled={saving}
					onCheckedChange={(v) => toggle('anonymousErrors', v)}
				/>
			</div>
			<div class="flex items-center justify-between">
				<div>
					<Label for="anon-metrics" class="text-sm">Anonymous usage metrics</Label>
					<p class="text-xs text-muted-foreground">
						Aggregated, anonymous events about how the app is used.
					</p>
				</div>
				<Switch
					id="anon-metrics"
					checked={consent.anonymousMetrics ?? false}
					disabled={saving}
					onCheckedChange={(v) => toggle('anonymousMetrics', v)}
				/>
			</div>
			<div class="flex items-center justify-between">
				<div>
					<Label for="non-anon-metrics" class="text-sm">Link metrics to my account</Label>
					<p class="text-xs text-muted-foreground">
						Associate usage events with your email so we can support you directly.
					</p>
				</div>
				<Switch
					id="non-anon-metrics"
					checked={consent.nonAnonymousMetrics ?? false}
					disabled={saving || !consent.anonymousMetrics}
					onCheckedChange={(v) => toggle('nonAnonymousMetrics', v)}
				/>
			</div>
		</section>

		<section class="flex flex-col gap-4">
			<h2 class="text-sm font-medium text-muted-foreground">Identifier</h2>
			<Separator />
			<div class="flex items-center justify-between gap-4">
				<div class="min-w-0 flex-1">
					<Label class="text-sm">Telemetry id</Label>
					<p class="truncate font-mono text-xs text-muted-foreground">
						{local.distinctId ?? 'not yet generated'}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<Button
						variant="outline"
						size="sm"
						disabled={!local.distinctId}
						onclick={copyId}
					>
						Copy
					</Button>
					<Button variant="outline" size="sm" disabled={rotating} onclick={rotateId}>
						{rotating ? 'Resetting…' : 'Reset'}
					</Button>
				</div>
			</div>
			<p class="text-xs text-muted-foreground">
				A random id used to group events from this install. Resetting it severs the link
				between past and future events.
			</p>
		</section>
	{/if}
</div>
