<script lang="ts">
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import CircleUserRoundIcon from '@lucide/svelte/icons/circle-user-round';
	import ServerIcon from '@lucide/svelte/icons/server';
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	const taurpc = createTauRPCProxy();
	let service_name: String | undefined = $state(undefined);

	onMount(() => {
		let unlisten: any;
		taurpc.prompt.prompt_service_change
			.on((name) => {
				service_name = name || undefined;
			})
			.then((unlistenFn) => {
				unlisten = unlistenFn;
			});
		return () => {
			unlisten?.();
		};
	});

	function disconnect() {
		taurpc.prompt.disconnect();
		goto('/onboarding');
	}
</script>

<div class="flex items-center justify-between p-4 h-[70px]">
	<div class="flex items-center gap-2">
		<a href="/onboarding" class="flex items-center gap-2">
			<EuroraLogo class="size-12" />
			Eurora AI
		</a>
	</div>
	<div class="flex items-center gap-2">
		{#if service_name}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="outline" class="flex items-center gap-2">
							<ServerIcon size="24px" />{service_name}</Button
						>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content>
					<DropdownMenu.Item onclick={disconnect}>Disconnect</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		{/if}
		<Button variant="ghost" size="icon">
			<CircleUserRoundIcon size="24px" class="size-6" />
		</Button>
	</div>
</div>
