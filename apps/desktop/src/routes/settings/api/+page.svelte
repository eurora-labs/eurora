<script lang="ts">
	import { type APISettings, type ConnectionMode } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as RadioGroup from '@eurora/ui/components/radio-group/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	type ModeKind = 'cloud' | 'local' | 'custom';

	let kind = $state<ModeKind>('cloud');
	let customUrl = $state('http://localhost:3000');
	let resolvedEndpoint = $state('');
	let testing = $state(false);
	let llmInfoText = $state<string | null>(null);

	function buildMode(): ConnectionMode {
		switch (kind) {
			case 'cloud':
				return { kind: 'cloud' };
			case 'local':
				return { kind: 'local' };
			case 'custom':
				return { kind: 'custom', url: customUrl };
		}
	}

	async function save() {
		try {
			const settings: APISettings = { mode: buildMode() };
			const result = await taurpc.settings.set_api_settings(settings);
			applyResult(result);
			toast.success('Connection settings saved');
		} catch (error) {
			toast.error(`Failed to save: ${error}`);
		}
	}

	async function testConnection() {
		testing = true;
		llmInfoText = null;
		try {
			const url = kind === 'custom' ? customUrl : resolveBakedUrl(kind);
			const info = await taurpc.system.test_backend_url(url);
			const chat = info.roles.chat;
			llmInfoText = `Connected — ${chat.provider} / ${chat.model}`;
			toast.success(llmInfoText);
		} catch (error) {
			toast.error(`Test failed: ${error}`);
		} finally {
			testing = false;
		}
	}

	function resolveBakedUrl(k: 'cloud' | 'local'): string {
		// Mirror the constants in `euro-settings::api_settings`. The backend
		// also resolves these on its end when persisting, but this lets the
		// "test connection" button preview the URL before saving.
		return k === 'cloud' ? 'https://api.eurora-labs.com' : 'http://localhost:3000';
	}

	function applyResult(settings: APISettings) {
		kind = settings.mode.kind;
		if (settings.mode.kind === 'custom') {
			customUrl = settings.mode.url;
		}
		resolvedEndpoint =
			settings.mode.kind === 'custom'
				? settings.mode.url
				: resolveBakedUrl(settings.mode.kind);
	}

	onMount(async () => {
		const settings = await taurpc.settings.get_api_settings();
		applyResult(settings);
	});
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">Connection</h1>
		<p class="text-sm text-muted-foreground">
			Choose which Eurora backend the desktop app talks to.
		</p>
	</div>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Backend</h2>
		<Separator />

		<RadioGroup.Root bind:value={kind} class="flex flex-col gap-3">
			<div class="flex items-start gap-3">
				<RadioGroup.Item value="cloud" id="mode-cloud" class="mt-1" />
				<Label for="mode-cloud" class="flex flex-col gap-1">
					<span class="text-sm font-medium">Eurora Cloud</span>
					<span class="text-xs text-muted-foreground">
						Hosted backend at api.eurora-labs.com.
					</span>
				</Label>
			</div>
			<div class="flex items-start gap-3">
				<RadioGroup.Item value="local" id="mode-local" class="mt-1" />
				<Label for="mode-local" class="flex flex-col gap-1">
					<span class="text-sm font-medium">Local</span>
					<span class="text-xs text-muted-foreground">
						Backend running on this machine at http://localhost:3000.
					</span>
				</Label>
			</div>
			<div class="flex items-start gap-3">
				<RadioGroup.Item value="custom" id="mode-custom" class="mt-1" />
				<Label for="mode-custom" class="flex flex-col gap-1 flex-1">
					<span class="text-sm font-medium">Custom</span>
					<Input
						placeholder="https://eurora.example.com"
						bind:value={customUrl}
						disabled={kind !== 'custom'}
						class="mt-1"
					/>
				</Label>
			</div>
		</RadioGroup.Root>

		<div class="flex items-center gap-2">
			<Button onclick={save}>Save</Button>
			<Button variant="outline" onclick={testConnection} disabled={testing}>
				{#if testing}
					<Spinner class="size-4" />
					Testing…
				{:else}
					Test connection
				{/if}
			</Button>
		</div>

		{#if resolvedEndpoint}
			<p class="text-xs text-muted-foreground">Active: {resolvedEndpoint}</p>
		{/if}
		{#if llmInfoText}
			<p class="text-xs text-muted-foreground">{llmInfoText}</p>
		{/if}
	</section>
</div>
