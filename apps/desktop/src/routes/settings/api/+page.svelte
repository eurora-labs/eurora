<script lang="ts">
	import { type APISettings, type ProviderSettings } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Collapsible from '@eurora/ui/components/collapsible/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import LoaderIcon from '@lucide/svelte/icons/loader';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	type ProviderType = 'cloud' | 'ollama' | 'openai';

	let providerType = $state<ProviderType>('cloud');
	let endpoint = $state('');
	let hasApiKey = $state(false);
	let connecting = $state(false);
	let connected = $state(false);
	let configOpen = $state(true);

	// Ollama fields
	let ollamaBaseUrl = $state('http://localhost:11434');
	let ollamaModel = $state('llama3.2');

	// OpenAI fields
	let openaiBaseUrl = $state('https://api.openai.com/v1');
	let openaiApiKey = $state('');
	let openaiModel = $state('gpt-4o');
	let openaiTitleModel = $state('');

	const triggerLabel = $derived(
		providerType === 'cloud'
			? 'Cloud (default)'
			: providerType === 'ollama'
				? 'Ollama'
				: 'OpenAI',
	);

	function detectProviderType(provider: ProviderSettings | null): ProviderType {
		if (!provider) return 'cloud';
		if ('OllamaSettings' in provider) return 'ollama';
		if ('OpenAISettings' in provider) return 'openai';
		return 'cloud';
	}

	function loadProviderFields(provider: ProviderSettings | null) {
		if (!provider) return;
		if ('OllamaSettings' in provider) {
			const s = provider.OllamaSettings;
			ollamaBaseUrl = s.base_url || 'http://localhost:11434';
			ollamaModel = s.model || 'llama3.2';
		} else if ('OpenAISettings' in provider) {
			const s = provider.OpenAISettings;
			openaiBaseUrl = s.base_url || 'https://api.openai.com/v1';
			openaiModel = s.model || 'gpt-4o';
			openaiTitleModel = s.title_model ?? '';
		}
	}

	function buildProvider(): ProviderSettings | null {
		if (providerType === 'ollama') {
			return {
				OllamaSettings: {
					base_url: ollamaBaseUrl,
					model: ollamaModel,
				},
			};
		}
		if (providerType === 'openai') {
			return {
				OpenAISettings: {
					base_url: openaiBaseUrl,
					model: openaiModel,
					title_model: openaiTitleModel || null,
				},
			};
		}
		return null;
	}

	async function connect() {
		connecting = true;
		try {
			// Persist OpenAI API key to system keyring if provided
			if (providerType === 'openai' && openaiApiKey) {
				await taurpc.third_party.save_api_key(openaiApiKey);
				hasApiKey = true;
				openaiApiKey = '';
			}

			const model = providerType === 'ollama' ? ollamaModel : 'llama3.2';
			const info = await taurpc.system.start_local_backend(model);
			endpoint = `http://localhost:${info.grpc_port}`;

			const settings: APISettings = {
				endpoint,
				provider: buildProvider(),
			};

			const result = await taurpc.settings.set_api_settings(settings);
			endpoint = result.endpoint;
			loadProviderFields(result.provider);

			try {
				await taurpc.auth.login('local@localhost', 'local');
			} catch {
				await taurpc.auth.register('local', 'local@localhost', 'local');
			}

			connected = true;
			toast.success('Backend started and connected');
		} catch (error) {
			toast.error(`Failed to connect: ${error}`);
		} finally {
			connecting = false;
		}
	}

	onMount(async () => {
		const [settings, keyExists] = await Promise.all([
			taurpc.settings.get_api_settings(),
			taurpc.third_party.check_api_key_exists(),
		]);

		hasApiKey = keyExists;
		providerType = detectProviderType(settings.provider);
		loadProviderFields(settings.provider);
		endpoint = settings.endpoint;
	});
</script>

<div class="flex flex-col p-4 gap-6">
	<h1 class="text-2xl font-semibold">API Settings</h1>

	<div class="flex flex-col gap-2">
		<Label>Provider</Label>
		<Select.Select type="single" bind:value={providerType}>
			<Select.SelectTrigger class="w-60">
				{triggerLabel}
			</Select.SelectTrigger>
			<Select.SelectContent>
				<Select.SelectItem value="cloud" label="Cloud (default)">
					Cloud (default)
				</Select.SelectItem>
				<Select.SelectItem value="ollama" label="Ollama">Ollama</Select.SelectItem>
				<Select.SelectItem value="openai" label="OpenAI">OpenAI</Select.SelectItem>
			</Select.SelectContent>
		</Select.Select>
	</div>

	{#if providerType === 'ollama'}
		<Collapsible.Collapsible bind:open={configOpen}>
			<Collapsible.CollapsibleTrigger class="flex items-center gap-2 text-sm font-medium">
				<ChevronDownIcon
					class="size-4 transition-transform {configOpen ? 'rotate-0' : '-rotate-90'}"
				/>
				Configuration
			</Collapsible.CollapsibleTrigger>
			<Collapsible.CollapsibleContent>
				<div class="flex flex-col gap-4 pt-3">
					<div class="flex flex-col gap-2">
						<Label for="ollama-base-url">Ollama base URL</Label>
						<Input
							id="ollama-base-url"
							placeholder="http://localhost:11434"
							bind:value={ollamaBaseUrl}
						/>
					</div>
					<div class="flex flex-col gap-2">
						<Label for="ollama-model">Model</Label>
						<Input id="ollama-model" placeholder="llama3.2" bind:value={ollamaModel} />
					</div>
				</div>
			</Collapsible.CollapsibleContent>
		</Collapsible.Collapsible>
	{/if}

	{#if providerType === 'openai'}
		<Collapsible.Collapsible bind:open={configOpen}>
			<Collapsible.CollapsibleTrigger class="flex items-center gap-2 text-sm font-medium">
				<ChevronDownIcon
					class="size-4 transition-transform {configOpen ? 'rotate-0' : '-rotate-90'}"
				/>
				Configuration
			</Collapsible.CollapsibleTrigger>
			<Collapsible.CollapsibleContent>
				<div class="flex flex-col gap-4 pt-3">
					<div class="flex flex-col gap-2">
						<Label for="openai-base-url">Base URL</Label>
						<Input
							id="openai-base-url"
							placeholder="https://api.openai.com/v1"
							bind:value={openaiBaseUrl}
						/>
					</div>
					<div class="flex flex-col gap-2">
						<Label for="openai-api-key">API key</Label>
						<Input
							id="openai-api-key"
							type="password"
							placeholder={hasApiKey ? 'Key is set (leave blank to keep)' : 'sk-...'}
							bind:value={openaiApiKey}
						/>
					</div>
					<div class="flex flex-col gap-2">
						<Label for="openai-model">Model</Label>
						<Input id="openai-model" placeholder="gpt-4o" bind:value={openaiModel} />
					</div>
					<div class="flex flex-col gap-2">
						<Label for="openai-title-model">Title model (optional)</Label>
						<Input
							id="openai-title-model"
							placeholder="gpt-4o-mini"
							bind:value={openaiTitleModel}
						/>
					</div>
				</div>
			</Collapsible.CollapsibleContent>
		</Collapsible.Collapsible>
	{/if}

	{#if providerType !== 'cloud'}
		<div class="flex items-center gap-3">
			<Button onclick={connect} disabled={connecting || connected}>
				{#if connecting}
					<LoaderIcon class="size-4 animate-spin" />
					Connecting...
				{:else if connected}
					<CheckIcon class="size-4" />
					Connected
				{:else}
					<PlayIcon class="size-4" />
					Connect
				{/if}
			</Button>
			{#if connected && endpoint}
				<span class="text-sm text-muted-foreground">{endpoint}</span>
			{/if}
		</div>
	{/if}
</div>
