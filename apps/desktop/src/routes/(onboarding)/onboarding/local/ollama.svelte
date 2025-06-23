<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';

	const taurpc = createTauRPCProxy();

	let ollamaUrl = $state('http://localhost:11434');
	let modelName = $state('llama3.2:latest');

	async function connect() {
		await taurpc.third_party.switch_to_ollama(ollamaUrl, modelName);
	}
</script>

<Card.Root class="flex-1 justify-between">
	<Card.Header>
		<Card.Title>Ollama Configuration</Card.Title>
	</Card.Header>
	<Card.Content class="space-y-4">
		<div class="space-y-2">
			<Label for="ollama-url">Ollama URL</Label>
			<Input
				id="ollama-url"
				type="text"
				placeholder="http://localhost:11434"
				bind:value={ollamaUrl}
			/>
		</div>

		<div class="space-y-2">
			<Label for="model-name">Model Name</Label>
			<Input
				id="model-name"
				type="text"
				placeholder="llama2, codellama, etc."
				bind:value={modelName}
			/>
		</div>
	</Card.Content>
	<Card.Footer class="flex justify-end">
		<Button class="flex items-center gap-2" onclick={connect}>Connect</Button>
	</Card.Footer>
</Card.Root>
