<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';

	let errorReporting = $state(true);
	let usageMetrics = $state(true);
	let nonAnonymousUsageMetrics = $state(false);
</script>

<div class="relative flex h-full w-full flex-col">
	<div class="flex flex-col justify-center items-left h-full w-full px-8">
		<article class="pb-4">
			<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Welcome to Eurora!</h1>
			<p class="pb-2">
				Eurora uses these metrics strictly to help us improve the app. We do not collect any
				personal information unless you yourself choose to provide it.
			</p>
			<p>
				I ask you to please keep these settings enabled. Eurora is self-funded and these
				metrics are essential for us to improve the app and stay competitive with
				billionaire-funded apps.
			</p>
		</article>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Error reporting</Item.Title>
				<Item.Description>A simple item with title and description.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={errorReporting} />
			</Item.Actions>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Usage metrics</Item.Title>
				<Item.Description>A simple item with title and description.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={usageMetrics} />
			</Item.Actions>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>Non-anonymous usage metrics</Item.Title>
				<Item.Description>A simple item with title and description.</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Switch bind:checked={nonAnonymousUsageMetrics} />
			</Item.Actions>
		</Item.Root>
		<div class="flex justify-end">
			<Button variant="default" onclick={() => goto('/onboarding/login')}
				>Continue
				<ChevronRight />
			</Button>
		</div>

		<!-- <Card.Root
			class="flex group cursor-pointer w-1/2"
			onclick={() => goto('/onboarding/first-party')}
		>
			<Card.Header class="pb-6 text-left ">
				<Card.Title class="mb-2 text-2xl font-semibold">Get started with Eurora</Card.Title>
				<Card.Description class="">Fastest way to get started.</Card.Description>
			</Card.Header>
		</Card.Root> -->
		<!-- First party login card -->
		<!-- <Button variant="ghost" onclick={() => goto('/onboarding/local')}
			>Or set up a local or a remote LLM provider</Button
		> -->
	</div>

	<!-- Footer space -->
	<div class="pb-16"></div>
</div>
