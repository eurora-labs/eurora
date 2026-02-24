<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import ServerIcon from '@lucide/svelte/icons/server';
</script>

<div>
	<h1 class="mb-2 text-3xl font-bold">Documentation</h1>
	<p class="mb-8 text-muted-foreground">Learn how to set up and use Eurora.</p>

	<div class="grid gap-4">
		<Card.Root class="group">
			<a href="/docs/self-hosting" class="block p-6">
				<Card.Header class="p-0 pb-2">
					<div class="flex items-center gap-2">
						<ServerIcon size={18} class="text-primary" />
						<Card.Title>Self-Hosting</Card.Title>
					</div>
				</Card.Header>
				<Card.Content class="p-0">
					<p class="text-sm text-muted-foreground">
						Run the Eurora backend on your own machine using Docker Compose with full
						control over your data and local models.
					</p>
				</Card.Content>
			</a>
		</Card.Root>
	</div>
</div>
