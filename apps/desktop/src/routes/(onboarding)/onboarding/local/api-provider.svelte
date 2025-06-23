<script lang="ts">
	import * as Select from '@eurora/ui/components/select/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';

	const providers = [
		{ value: 'openai', label: 'OpenAI' },
		{ value: 'anthropic', label: 'Anthropic' },
		{ value: 'openrouter', label: 'OpenRouter' },
	];

	const models = {
		openai: [{ value: 'gpt-3.5-turbo', label: 'GPT-3.5 Turbo' }],
		anthropic: [{ value: 'claude-3-5-sonnet', label: 'Claude 3.5 Sonnet' }],
		openrouter: [{ value: 'llama3.2', label: 'Llama 3.2' }],
	};

	let apiProvider = $state<string>('');
	let apiKey = $state('');
	let model = $state<string>('');

	const triggerContent = $derived(
		providers.find((f) => f.value === apiProvider)?.label ?? 'Select provider',
	);

	const modelTriggerContent = $derived(
		models[apiProvider as keyof typeof models]?.find((f) => f.value === model)?.label ??
			'Select model',
	);
</script>

<Card.Root>
	<Card.Header>
		<Card.Title>Remote Provider</Card.Title>
	</Card.Header>

	<Card.Content class="space-y-4">
		<Label for="api-provider">Provider</Label>
		<Select.Root type="single" name="api-provider" bind:value={apiProvider}>
			<Select.Trigger class="w-[180px]">
				{triggerContent}
			</Select.Trigger>
			<Select.Content>
				<Select.Group>
					<Select.Label>API Provider</Select.Label>
					{#each providers as provider (provider.value)}
						<Select.Item value={provider.value} label={provider.label}>
							{provider.label}
						</Select.Item>
					{/each}
				</Select.Group>
			</Select.Content>
		</Select.Root>
		<Label for="api-key">API Key</Label>
		<Input id="api-key" type="password" placeholder="Enter your API key" bind:value={apiKey} />
		<Label for="model">Model</Label>
		<Select.Root type="single" name="model" bind:value={model}>
			<Select.Trigger class="w-[180px]">
				{modelTriggerContent}
			</Select.Trigger>
			<Select.Content>
				<Select.Group>
					<Select.Label>Model</Select.Label>
					{#each models[apiProvider as keyof typeof models] as model (model.value)}
						<Select.Item value={model.value} label={model.label}>
							{model.label}
						</Select.Item>
					{/each}
				</Select.Group>
			</Select.Content>
		</Select.Root>
	</Card.Content>
</Card.Root>
