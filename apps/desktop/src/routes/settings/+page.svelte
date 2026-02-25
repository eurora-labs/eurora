<script lang="ts">
	import { type GeneralSettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);

	let generalSettings = $state<GeneralSettings | null>(null);
	let autostartEnabled = $state(false);
	let authenticated = $state(false);
	let username = $state('');
	let email = $state('');
	let role = $state('');

	const planLabel = $derived(role === 'Tier1' ? 'Pro' : 'Free');

	async function saveSettings() {
		await taurpc.settings.set_general_settings({
			...generalSettings,
			autostart: autostartEnabled,
		});
	}

	onMount(async () => {
		const [settings, isAuth] = await Promise.all([
			taurpc.settings.get_general_settings(),
			taurpc.auth.is_authenticated(),
		]);

		generalSettings = settings;
		autostartEnabled = generalSettings.autostart;
		authenticated = isAuth;

		if (authenticated) {
			const [u, e, r] = await Promise.all([
				taurpc.auth.get_username(),
				taurpc.auth.get_email(),
				taurpc.auth.get_role(),
			]);
			username = u;
			email = e;
			role = r;
		}
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
		{#if !authenticated}
			<FirstPartyLogin />
		{:else}
			<div class="flex items-center justify-between">
				<span class="text-sm">Name</span>
				<Input class="max-w-60" disabled value={username} />
			</div>
			<div class="flex items-center justify-between">
				<span class="text-sm">Email</span>
				<Input class="max-w-60" disabled value={email} />
			</div>
			<div class="flex items-center justify-between">
				<span class="text-sm">Plan</span>
				<Badge variant={planLabel === 'Pro' ? 'default' : 'secondary'}>{planLabel}</Badge>
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
