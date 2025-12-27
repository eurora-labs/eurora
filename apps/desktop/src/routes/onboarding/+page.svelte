<script lang="ts">
	import { goto } from '$app/navigation';
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
</script>

<div class="relative flex h-full w-full flex-col">
	<div class="flex flex-col justify-center items-center h-full w-full gap-8 px-8">
		<h1 class="text-4xl font-bold drop-shadow-lg">Welcome to Eurora!</h1>
		<Card.Root
			class="flex group cursor-pointer w-1/2"
			onclick={() => goto('/onboarding/first-party')}
		>
			<Card.Header class="pb-6 text-left ">
				<Card.Title class="mb-2 text-2xl font-semibold">Get started with Eurora</Card.Title>
				<Card.Description class="">Fastest way to get started.</Card.Description>
			</Card.Header>
			<!-- <Card.Content class="flex justify-center">Easiest and fastest way to get started.</Card.Content> -->
		</Card.Root>
		<!-- First party login card -->
		<Button variant="ghost" onclick={() => goto('/onboarding/local')}
			>Or set up a local or a remote LLM provider</Button
		>
	</div>

	<!-- Footer space -->
	<div class="pb-16"></div>
</div>
