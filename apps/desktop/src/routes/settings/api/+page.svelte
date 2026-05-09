<script lang="ts">
	import { unwrap } from '$lib/bindings/result.js';
	import {
		commands,
		type APISettings,
		type ConnectionMode,
	} from '$lib/bindings/specta.bindings.js';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as RadioGroup from '@eurora/ui/components/radio-group/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	type ModeKind = 'default' | 'custom';

	let kind = $state<ModeKind>('default');
	let customUrl = $state('http://localhost:3000');
	// The backend URL `Default` mode resolves to. Baked at compile time
	// from `BACKEND_URL` and fetched via `get_default_backend_url` so the
	// UI never lies about what the binary will actually connect to.
	let defaultUrl = $state('');
	let resolvedEndpoint = $state('');
	let testing = $state(false);
	let llmInfoText = $state<string | null>(null);

	function buildMode(): ConnectionMode {
		switch (kind) {
			case 'default':
				return { kind: 'default' };
			case 'custom':
				return { kind: 'custom', url: customUrl };
		}
	}

	async function save() {
		try {
			const settings: APISettings = { mode: buildMode() };
			const result = unwrap(await commands.settingsSetApi(settings));
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
			const url = kind === 'custom' ? customUrl : defaultUrl;
			const info = unwrap(await commands.systemTestBackendUrl(url));
			const chat = info.roles.chat;
			llmInfoText = `Connected — ${chat.provider} / ${chat.model}`;
			toast.success(llmInfoText);
		} catch (error) {
			toast.error(`Test failed: ${error}`);
		} finally {
			testing = false;
		}
	}

	function applyResult(settings: APISettings) {
		kind = settings.mode.kind;
		if (settings.mode.kind === 'custom') {
			customUrl = settings.mode.url;
		}
		resolvedEndpoint = settings.mode.kind === 'custom' ? settings.mode.url : defaultUrl;
	}

	onMount(async () => {
		// Order matters: the baked default URL has to land first so
		// `applyResult` can fold it into `resolvedEndpoint` for the
		// "Active: …" hint.
		defaultUrl = await commands.systemGetDefaultBackendUrl();
		const settings = await commands.settingsGetApi();
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
				<RadioGroup.Item value="default" id="mode-default" class="mt-1" />
				<div class="flex flex-col gap-0.5">
					<Label for="mode-default" class="text-sm font-medium">Default</Label>
					<span class="text-xs text-muted-foreground">
						{defaultUrl || 'Loading…'}
					</span>
				</div>
			</div>
			<div class="flex items-start gap-3">
				<RadioGroup.Item value="custom" id="mode-custom" class="mt-1" />
				<div class="flex flex-1 flex-col gap-2">
					<Label for="mode-custom" class="text-sm font-medium">Custom</Label>
					<Input
						placeholder="https://eurora.example.com"
						bind:value={customUrl}
						disabled={kind !== 'custom'}
					/>
				</div>
			</div>
		</RadioGroup.Root>

		<div class="flex flex-col gap-2">
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
		</div>
	</section>
</div>
