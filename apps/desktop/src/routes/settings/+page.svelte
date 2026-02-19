<script lang="ts">
	import { type GeneralSettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
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

<div class="flex flex-col p-4 gap-4">
	<h1 class="text-2xl font-semibold">General</h1>

	{#if !authenticated}
		<FirstPartyLogin />
	{:else}
		<div class="flex flex-col gap-4">
			<div class="flex items-center gap-2">
				<Label for="name">Name</Label>
				<Input id="name" disabled value={username} />
			</div>

			<div class="flex items-center gap-2">
				<Label for="email">Email</Label>
				<Input id="email" disabled value={email} />
			</div>

			<div class="flex items-center gap-2">
				<Label>Plan</Label>
				<Badge variant={planLabel === 'Pro' ? 'default' : 'secondary'}>{planLabel}</Badge>
			</div>
		</div>
	{/if}

	<div class="flex items-center gap-2">
		<Switch id="autostart" bind:checked={autostartEnabled} onCheckedChange={saveSettings} />
		<Label for="autostart">Enable autostart</Label>
	</div>
</div>
