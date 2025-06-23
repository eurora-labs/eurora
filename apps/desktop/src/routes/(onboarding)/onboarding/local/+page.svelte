<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Card, CardContent, CardHeader, CardTitle } from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';
	import Ollama from './ollama.svelte';
	import ApiProvider from './api-provider.svelte';

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
		<Ollama />
	</div>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
	</div>
</div>
