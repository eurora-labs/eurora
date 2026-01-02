<script lang="ts">
	import { goto } from '$app/navigation';
	import * as Card from '@eurora/ui/components/card/index';
</script>

<div class="flex flex-col justify-center items-center h-full p-8">
	<Card.Root
		class="flex group cursor-pointer w-full"
		onclick={() => goto('/onboarding/first-party')}
	>
		<Card.Header class="pb-6 text-left ">
			<Card.Title class="mb-2 text-2xl font-semibold">Log In/Sign Up</Card.Title>
			<Card.Description>Login to access the Eurora app</Card.Description>
		</Card.Header>
	</Card.Root>
</div>
