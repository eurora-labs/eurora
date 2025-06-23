<script lang="ts">
	import ApiProvider from './api-provider.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import { Card, CardContent, CardHeader, CardTitle } from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';

	let apiProvider = $state('');
	let apiKey = $state('');
	let ollamaUrl = $state('');
	let modelName = $state('');
</script>

<div class="w-full h-screen mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Local AI Configuration</h1>

	<div class="grid grid-cols-2 lg:grid-cols-2 gap-6 flex-1">
		<!-- Left Column: API Provider Configuration -->
		<!-- <Card>
			<CardHeader>
				<CardTitle>API Provider</CardTitle>
			</CardHeader>
			<CardContent class="space-y-4"> -->
		<ApiProvider />
		<!-- <div class="space-y-2">
					<Label for="api-provider">Provider</Label>
					<Select.Root type="single" bind:value={apiProvider}>
						<Select.Trigger
							id="api-provider"
							class="w-full"
							placeholder="Select API provider"
						></Select.Trigger>
						<Select.Content>
							<Select.Item value="openai">OpenAI</Select.Item>
							<Select.Item value="anthropic">Anthropic</Select.Item>
							<Select.Item value="google">Google</Select.Item>
							<Select.Item value="azure">Azure OpenAI</Select.Item>
							<Select.Item value="ollama">Ollama</Select.Item>
						</Select.Content>
					</Select.Root>
				</div>

				<div class="space-y-2">
					<Label for="api-key">API Key</Label>
					<Input
						id="api-key"
						type="password"
						placeholder="Enter your API key"
						bind:value={apiKey}
					/>
				</div>
			</CardContent>
		</Card> -->

		<!-- Right Column: Ollama Configuration -->
		<Card>
			<CardHeader>
				<CardTitle>Ollama Configuration</CardTitle>
			</CardHeader>
			<CardContent class="space-y-4">
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
			</CardContent>
		</Card>
	</div>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
		<Button>Continue</Button>
	</div>
</div>
