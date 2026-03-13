<script lang="ts">
	import { type GeneralSettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);
	const user = inject(USER_SERVICE);

	let generalSettings = $state<GeneralSettings | null>(null);
	let autostartEnabled = $state(false);

	async function saveSettings() {
		await taurpc.settings.set_general_settings({
			...generalSettings,
			autostart: autostartEnabled,
		});
	}

	onMount(async () => {
		generalSettings = await taurpc.settings.get_general_settings();
		autostartEnabled = generalSettings.autostart;
	});
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">General</h1>
		<p class="text-sm text-muted-foreground">Account and application preferences.</p>
	</div>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Account</h2>
		<Separator />
		{#if !user.authenticated}
			<FirstPartyLogin />
		{:else}
			<div class="flex items-center justify-between">
				<span class="text-sm">Name</span>
				<Input class="max-w-60" disabled value={user.username} />
			</div>
			<div class="flex items-center justify-between">
				<span class="text-sm">Email</span>
				<Input class="max-w-60" disabled value={user.email} />
			</div>
			<div class="flex items-center justify-between">
				<span class="text-sm">Plan</span>
				<Badge variant={user.planLabel === 'Pro' ? 'default' : 'secondary'}
					>{user.planLabel}</Badge
				>
			</div>
		{/if}
	</section>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Application</h2>
		<Separator />
		<div class="flex items-center justify-between">
			<Label for="autostart" class="text-sm">Launch at startup</Label>
			<Switch id="autostart" bind:checked={autostartEnabled} onCheckedChange={saveSettings} />
		</div>
	</section>
</div>
