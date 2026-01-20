<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import ServerIcon from '@lucide/svelte/icons/server';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);
	let service_name: string | undefined = $state(undefined);

	onMount(() => {
		taurpc.auth
			.is_authenticated()
			.then((isAuthenticated) => {
				if (!isAuthenticated) {
					goto('/onboarding');
				}
			})
			.catch((error) => {
				goto('/onboarding');
				console.error('Failed to check authentication:', error);
			});
	});

	function disconnect() {
		taurpc.prompt.disconnect();
		goto('/onboarding');
	}
</script>

<div class="flex items-center justify-end p-4 h-17.5">
	<div class="flex items-center gap-2">
		{#if service_name}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" class="flex items-center gap-2">
							<ServerIcon size="24px" />{service_name}</Button
						>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content>
					<DropdownMenu.Item onclick={disconnect}>Disconnect</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
	</div>
</div>
