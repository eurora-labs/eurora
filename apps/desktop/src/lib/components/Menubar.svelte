<script lang="ts">
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import CircleUserRoundIcon from '@lucide/svelte/icons/circle-user-round';
	import ServerIcon from '@lucide/svelte/icons/server';
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
</script>

<div class="flex items-center justify-between p-4 h-[70px]">
	<div class="flex items-center gap-2">
		<EuroraLogo class="size-12" />
		Eurora AI
	</div>
	<div class="flex items-center gap-2">
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="outline" class="flex items-center gap-2">
						<ServerIcon size="24px" />llama:3.2</Button
					>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content>
				<DropdownMenu.Item>Disconnect</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
		<Button variant="ghost" size="icon">
			<CircleUserRoundIcon size="24px" class="size-6" />
		</Button>
	</div>
</div>
