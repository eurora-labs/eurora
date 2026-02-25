<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { open } from '@tauri-apps/plugin-shell';
	import { toast } from 'svelte-sonner';
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

		<Button size="lg" class="w-fit" onclick={() => goto('/no-access/upgrade')}>
			Upgrade Plan
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
