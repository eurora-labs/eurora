<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
</script>

<div class="flex flex-col justify-center items-center h-full p-8 gap-6">
	<Card.Root class="w-full cursor-pointer" onclick={() => goto('/onboarding/login/first-party')}>
		<Card.Header class="pb-2 text-left">
			<Card.Title class="mb-2 text-3xl font-bold">Log In / Sign Up</Card.Title>
			<Card.Description class="text-base">
				Access everything Eurora has to offer. Enjoy fully private European Soveign AI.
			</Card.Description>
		</Card.Header>
	</Card.Root>

	<div class="flex items-center w-full gap-4">
		<div class="h-px flex-1 bg-border"></div>
		<span class="text-sm text-muted-foreground">or</span>
		<div class="h-px flex-1 bg-border"></div>
	</div>

	<Button variant="ghost" onclick={() => goto('/onboarding/login/local')}>
		Run locally with your own models
	</Button>
</div>
