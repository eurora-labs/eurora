<script lang="ts">
	import { currentUser } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
</script>

<svelte:head>
	<title>{$currentUser?.name || 'User'} - Eurora Labs</title>
</svelte:head>

{#if $currentUser}
	<Card.Root
		class="relative overflow-hidden bg-linear-to-r from-primary/5 via-primary/10 to-transparent p-6"
	>
		<div class="flex items-center justify-between gap-4">
			<div class="flex items-start gap-4">
				<div
					class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary/10"
				>
					<SparklesIcon size={20} class="text-primary" />
				</div>
				<div>
					<h3 class="text-base font-semibold">Upgrade to Pro</h3>
					<p class="mt-1 text-sm text-muted-foreground">
						Unlock advanced features, priority support, and more with the Pro plan.
					</p>
				</div>
			</div>
			<Button href="/settings/billing" class="shrink-0">Upgrade</Button>
		</div>
	</Card.Root>

	<div class="mt-6">
		<Button variant="outline" class="gap-2">
			Download for Linux
			<DownloadIcon size={16} />
		</Button>
	</div>
{:else}
	<Card.Root class="p-6">
		<p class="text-muted-foreground text-sm">
			Please <a href="/login" class="underline">sign in</a> to view your profile.
		</p>
	</Card.Root>
{/if}
