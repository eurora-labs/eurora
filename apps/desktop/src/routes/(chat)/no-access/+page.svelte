<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import { open } from '@tauri-apps/plugin-shell';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let loading = $state(false);

	async function handleUpgrade() {
		loading = true;
		try {
			const url = await taurpc.payment.create_checkout_url();
			await open(url);
			goto('/no-access/upgrade');
		} catch (e) {
			toast.error(`Failed to start checkout: ${e}`);
			loading = false;
		}
	}
</script>

<div class="flex flex-col justify-center items-center h-full p-8">
	<div class="flex flex-col max-w-md gap-6">
		<div>
			<h1 class="text-3xl font-bold pb-2">Subscription Required</h1>
			<p class="text-muted-foreground">
				Your current plan does not include cloud access. Upgrade your plan or run Eurora
				locally with your own models.
			</p>
		</div>

		<Button size="lg" class="w-fit" onclick={handleUpgrade} disabled={loading}>
			{#if loading}
				<Loader2Icon class="size-4 animate-spin" />
				Redirecting...
			{:else}
				Upgrade Plan
			{/if}
		</Button>

		<Separator />

		<Button
			variant="link"
			class="w-fit text-muted-foreground"
			onclick={() =>
				open('https://www.eurora-labs.com/docs/self-hosting').catch((e) =>
					toast.error(`Failed to open link: ${e}`),
				)}
		>
			Run locally with your own models
			<ExternalLink class="size-3.5" />
		</Button>
	</div>
</div>
