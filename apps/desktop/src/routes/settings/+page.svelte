<script lang="ts">
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { GENERAL_SERVICE } from '$lib/services/general-service.svelte.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { toast } from 'svelte-sonner';

	const user = inject(USER_SERVICE);
	const general = inject(GENERAL_SERVICE);

	async function onAutostartChange(checked: boolean) {
		try {
			await general.setAutostart(checked);
		} catch (error) {
			toast.error(`Failed to update startup preference: ${error}`);
		}
	}
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
				<Input class="max-w-60" disabled value={user.displayName ?? ''} />
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
			<Switch
				id="autostart"
				checked={general.autostart}
				onCheckedChange={onAutostartChange}
			/>
		</div>
	</section>
</div>
