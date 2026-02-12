<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import PlayIcon from '@lucide/svelte/icons/play';
	import LoaderIcon from '@lucide/svelte/icons/loader';
	import { toast } from 'svelte-sonner';

	let taurpc = inject(TAURPC_SERVICE);

	let starting = $state(false);
	let backendInfo: { grpc_port: number; http_port: number; postgres_port: number } | null =
		$state(null);

	async function startBackend() {
		starting = true;
		try {
			const info = await taurpc.system.start_local_backend();
			backendInfo = info;
			toast.success(`Backend started on gRPC :${info.grpc_port}, HTTP :${info.http_port}`);
		} catch (error) {
			toast.error(`Failed to start backend: ${error}`);
		} finally {
			starting = false;
		}
	}
</script>

<div class="flex flex-col justify-center items-start h-full p-8">
	<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Run Locally</h1>
	<p class="text-muted-foreground pb-6">
		Set up the Eurora backend on your own machine using Docker.
	</p>

	<div class="w-full flex-1 overflow-y-auto pb-8">
		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>1. Make sure Docker is installed and running</Item.Title>
				<Item.Description>
					You can download Docker from the
					<a href="https://docs.docker.com/get-docker/" target="_blank"
						>official website</a
					>.
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>2. Start the backend</Item.Title>
				<Item.Description>
					This will start the Eurora backend and a PostgreSQL database using Docker
					Compose.
				</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Button onclick={startBackend} disabled={starting || !!backendInfo}>
					{#if starting}
						<LoaderIcon class="size-4 animate-spin" />
						Starting...
					{:else if backendInfo}
						<CheckIcon class="size-4" />
						Started
					{:else}
						<PlayIcon class="size-4" />
						Start
					{/if}
				</Button>
			</Item.Actions>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>3. Configure Eurora</Item.Title>
				<Item.Description>
					Once the backend is running, you can change the backend URL later in Settings
					&rarr; API.
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="outline">
			<Item.Content>
				<Item.Title>4. Check the connection</Item.Title>
				<Item.Description>
					Verify that the backend is reachable and enter the app.
				</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Button onclick={() => goto('/')}>
					<CheckIcon class="size-4" />
					Check
				</Button>
			</Item.Actions>
		</Item.Root>
	</div>

	<div class="pt-8">
		<Button variant="default" onclick={() => goto('/onboarding/no-access')}>Back</Button>
	</div>
</div>
