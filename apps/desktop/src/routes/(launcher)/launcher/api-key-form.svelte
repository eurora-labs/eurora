<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { invoke } from '@tauri-apps/api/core';
	import { createTauRPCProxy } from '@eurora/tauri-bindings';
	import { onMount } from 'svelte';

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();

	// State variables
	let apiKey = $state('');
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let hasApiKey = $state(false);

	let { saved } = $props();

	// Check if API key exists on mount
	onMount(async () => {
		try {
			const result = await taurpc.third_party.check_api_key_exists();
			hasApiKey = result;
		} catch (err) {
			console.error('Failed to check API key:', err);
			error = 'Failed to check if API key exists';
		}
	});

	// Save API key to keyring
	async function saveApiKey() {
		if (!apiKey.trim()) {
			error = 'Please enter a valid API key';
			return;
		}

		isLoading = true;
		error = null;

		try {
			// Save the API key to the keyring using TauRPC
			await taurpc.third_party.save_api_key(apiKey);

			// Initialize the OpenAI client with the new key using TauRPC
			await taurpc.third_party.initialize_openai_client();

			// Update state and notify parent
			hasApiKey = true;
			saved();
		} catch (err) {
			console.error('Failed to save API key:', err);
			error = 'Failed to save API key';
		} finally {
			isLoading = false;
		}
	}
</script>

<div class="mx-auto w-full max-w-md rounded-lg bg-white p-6 shadow-md">
	<div class="mb-4">
		<h2 class="mb-2 text-xl font-bold">Welcome to Eurora</h2>
		<p class="text-gray-600">
			Please enter your OpenAI API key to get started. Your key will be stored securely in your
			system's keyring.
		</p>
	</div>

	<div class="mb-6">
		<form onsubmit={saveApiKey} class="space-y-4">
			<div class="space-y-2">
				<Input type="password" placeholder="sk-..." bind:value={apiKey} class="w-full" />
				{#if error}
					<p class="text-sm text-red-500">{error}</p>
				{/if}
				<p class="text-xs text-gray-500">
					Your API key is stored securely and is only used to communicate with OpenAI's services.
				</p>
			</div>
		</form>
	</div>

	<div class="flex justify-end">
		<Button disabled={isLoading} onclick={() => saveApiKey()}>
			{isLoading ? 'Saving...' : 'Save API Key'}
		</Button>
	</div>
</div>
