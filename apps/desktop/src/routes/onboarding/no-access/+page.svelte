<script lang="ts">
	import { goto } from '$app/navigation';
	import * as Card from '@eurora/ui/components/card/index';
	import { open } from '@tauri-apps/plugin-shell';
	import { toast } from 'svelte-sonner';

	const pricingUrl = 'https://www.eurora-labs.com/pricing';

	async function openPricing() {
		try {
			await open(pricingUrl);
		} catch {
			toast.error(`Could not open browser. Please visit: ${pricingUrl}`);
		}
	}
</script>

<div class="flex flex-col justify-center items-center h-full p-8 gap-4">
	<div class="pb-4">
		<h1 class="text-4xl font-bold drop-shadow-lg pb-2">Subscription Required</h1>
		<p class="text-muted-foreground">
			Your current plan does not include cloud access. Upgrade your plan or run Eurora locally
			with your own models.
		</p>
	</div>

	<Card.Root class="flex group cursor-pointer w-full" onclick={openPricing}>
		<Card.Header class="pb-6 text-left">
			<Card.Title class="mb-2 text-2xl font-semibold">Upgrade Plan</Card.Title>
			<Card.Description>
				Get access to cloud features and all Eurora has to offer.
			</Card.Description>
		</Card.Header>
	</Card.Root>

	<Card.Root
		class="flex group cursor-pointer w-full"
		onclick={() => goto('/onboarding/login/local')}
	>
		<Card.Header class="pb-6 text-left">
			<Card.Title class="mb-2 text-2xl font-semibold">Run Locally</Card.Title>
			<Card.Description>
				Use Eurora with your own local models without a subscription.
			</Card.Description>
		</Card.Header>
	</Card.Root>
</div>
