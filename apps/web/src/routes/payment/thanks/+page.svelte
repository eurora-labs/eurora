<script lang="ts">
	import { page } from '$app/state';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import CheckIcon from '@lucide/svelte/icons/circle-check';

	const sessionId = page.url.searchParams.get('session_id');
</script>

<div class="flex min-h-[60vh] items-center justify-center px-4">
	<Card.Root class="max-w-md p-8 text-center">
		<Card.Header>
			<div
				class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100"
			>
				<CheckIcon class="h-8 w-8 text-green-600" />
			</div>
			<Card.Title class="text-2xl">Payment Successful!</Card.Title>
			<Card.Description>
				Your Pro subscription is now active. Thank you for your purchase.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="mt-6 space-y-3">
				<Button class="w-full" href="/settings">Go to Settings</Button>
				<Button variant="outline" class="w-full" href="/">Back to Home</Button>
			</div>
		</Card.Content>
	</Card.Root>
</div>
