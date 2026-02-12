<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Item from '@eurora/ui/components/item/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';

	const dockerRun = `docker run -d \\
  --name eurora-backend \\
  -p 8080:8080 \\
  ghcr.io/eurora-labs/eurora/be-monolith:latest \\
  --LOCAL \\
  --OLLAMA_MODEL=your_model \\
  --POSTGRESQL_URL=your_local_postgresql`;

	function copy(text: string) {
		navigator.clipboard.writeText(text);
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
				<Item.Title>1. Make sure Docker is installed</Item.Title>
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
				<Item.Title>2. Pull the official Eurora backend image</Item.Title>
				<Item.Description>
					<Button
						variant="ghost"
						class="font-mono text-xs"
						onclick={() =>
							copy('docker pull ghcr.io/eurora-labs/eurora/be-monolith:latest')}
					>
						docker pull ghcr.io/eurora-labs/eurora/be-monolith:latest
						<CopyIcon class="size-3.5 shrink-0" />
					</Button>
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>3. Start the backend</Item.Title>
				<Item.Description class="line-clamp-none">
					<span>
						Run the container with the required flags. Replace <code>your_model</code>
						with your Ollama model name and <code>your_local_postgresql</code> with your PostgreSQL
						connection string.
					</span>
					<br />
					<Button
						variant="ghost"
						class="font-mono text-xs whitespace-pre-wrap text-left h-auto py-2"
						onclick={() => copy(dockerRun)}
					>
						{dockerRun}
						<CopyIcon class="size-3.5 shrink-0 self-start mt-0.5" />
					</Button>
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>4. Configure Eurora</Item.Title>
				<Item.Description>
					Once the backend is running, you can change the backend URL later in Settings
					&rarr; API.
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="outline">
			<Item.Content>
				<Item.Title>5. Check the connection</Item.Title>
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
