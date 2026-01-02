<script lang="ts">
	import { goto } from '$app/navigation';
	import * as Card from '@eurora/ui/components/card/index';
</script>

<div class="flex flex-col justify-center items-center h-full p-8">
	<Card.Root
		class="flex group cursor-pointer w-full"
		onclick={() => goto('/onboarding/login/first-party')}
	>
		<Card.Header class="pb-6 text-left ">
			<Card.Title class="mb-2 text-2xl font-semibold">Log In / Sign Up</Card.Title>
			<Card.Description>Access everything Eurora has to offer.</Card.Description>
		</Card.Header>
	</Card.Root>
</div>
