<script lang="ts" module>
	export interface ApiProviderProps {
		finished?: () => void;
	}
</script>

<script lang="ts">
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import Button from '@eurora/ui/components/button/button.svelte';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';
	import CheckIcon from '@lucide/svelte/icons/check';

	let { finished }: ApiProviderProps = $props();

	const tauRPC = createTauRPCProxy();
	const providers = [
		{ value: 'openai', label: 'OpenAI' },
		{ value: 'anthropic', label: 'Anthropic' },
	];

	const models = {
		openai: [{ value: 'gpt-4o-2024-11-20', label: 'GPT-4o Latest' }],
		anthropic: [{ value: 'claude-sonnet-4-20250514', label: 'Claude 4.0 Sonnet' }],
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

	let isConnecting = $state(false);
	let connectionStatus = $state<'success' | 'error' | 'pending'>('pending');

	async function connect() {
		isConnecting = true;
		connectionStatus = 'pending';

		try {
			await tauRPC.prompt.switch_to_remote(apiProvider, apiKey, model);
			connectionStatus = 'success';
			finished?.();
		} catch (_error) {
			connectionStatus = 'error';
		} finally {
			isConnecting = false;
		}
	}
</script>

<Card.Root class="flex-1 justify-between">
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
	<Card.Footer class="flex justify-end">
		<Button
			variant="default"
			onclick={connect}
			disabled={isConnecting || connectionStatus === 'success'}
		>
			{#if connectionStatus === 'success'}
				<CheckIcon />
			{:else if connectionStatus === 'error'}
				Error Connecting
			{:else}
				{isConnecting ? 'Connecting...' : 'Connect'}
			{/if}
		</Button>
	</Card.Footer>
</Card.Root>
