<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
</script>

<div class="flex flex-col justify-center items-start h-full p-8">
	<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Run Locally</h1>

	<div class="w-full pb-8">
		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Make sure Docker is installed</Item.Title>
				<Item.Description>
					You can download Docker from the official website.
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Pull the official Eurora backend image</Item.Title>
				<Item.Description>
					<span class="text-muted-foreground"
						>Run the following command to pull the official Eurora backend image:</span
					>
					<Button
						variant="ghost"
						onclick={() =>
							navigator.clipboard.writeText(
								'docker pull ghcr.io/eurora-labs/eurora/be-monolith:latest',
							)}
					>
						<CopyIcon />
						<div class="w-fit h-full pr-4">
							docker pull ghcr.io/eurora-labs/eurora/be-monolith:latest
						</div>
					</Button>
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Configure Eurora</Item.Title>
				<Item.Description>Connect Eurora to your local provider.</Item.Description>
			</Item.Content>
		</Item.Root>
	</div>

	<Button variant="outline" onclick={() => goto('/onboarding/no-access')}>Back</Button>
</div>
