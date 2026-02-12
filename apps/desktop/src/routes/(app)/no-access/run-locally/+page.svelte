<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
</script>

<div class="flex flex-col justify-center items-start h-full p-8">
	<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Run Locally</h1>
	<p class="text-muted-foreground pb-6">
		To run Eurora locally, you'll need to set up a local model provider such as Ollama. More
		detailed instructions will be available here soon.
	</p>

	<div class="w-full pb-8">
		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Install a local model provider</Item.Title>
				<Item.Description>For example, install Ollama on your machine.</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Download a compatible model</Item.Title>
				<Item.Description>Pull a model that works with Eurora.</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Configure Eurora</Item.Title>
				<Item.Description>Connect Eurora to your local provider.</Item.Description>
			</Item.Content>
		</Item.Root>
	</div>

	<Button variant="outline" onclick={() => goto('/no-access')}>Back</Button>
</div>
