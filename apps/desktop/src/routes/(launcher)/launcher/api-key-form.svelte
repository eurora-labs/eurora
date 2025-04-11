<script lang="ts">
	import { Button, Input } from '@eurora/ui';
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	// import { createEventDispatcher } from 'svelte';

	// State variables
	let apiKey = $state('');
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let hasApiKey = $state(false);

	// Event emitter for when the API key is saved
	// const dispatch = createEventDispatcher();

    let {saved} = $props();

	// Check if API key exists on mount
	onMount(async () => {
		try {
			const result: { has_key: boolean } = await invoke('check_api_key_exists');
			hasApiKey = result.has_key;
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
			// Save the API key to the keyring
			await invoke('save_api_key', { apiKey });
			
			// Initialize the OpenAI client with the new key
			await invoke('initialize_openai_client');
			
			// Update state and notify parent
			hasApiKey = true;
            saved(true);
            saved();
			// dispatch('saved', true);
			// dispatch('saved');
		} catch (err) {
			console.error('Failed to save API key:', err);
			error = 'Failed to save API key';
		} finally {
			isLoading = false;
		}
	}
</script>

<div class="w-full max-w-md mx-auto bg-white rounded-lg shadow-md p-6">
	<div class="mb-4">
		<h2 class="text-xl font-bold mb-2">Welcome to Eurora</h2>
		<p class="text-gray-600">
			Please enter your OpenAI API key to get started. Your key will be stored securely in your system's keyring.
		</p>
	</div>
	
	<div class="mb-6">
		<form on:submit|preventDefault={saveApiKey} class="space-y-4">
			<div class="space-y-2">
				<Input
					type="password"
					placeholder="sk-..."
					bind:value={apiKey}
					class="w-full"
				/>
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
		<!-- <button
			class="bg-blue-500 hover:bg-blue-600 text-white font-medium py-2 px-4 rounded disabled:opacity-50"
			disabled={isLoading}
			on:click={() => saveApiKey()}
		>
			{isLoading ? 'Saving...' : 'Save API Key'}
		</button> -->
	</div>
</div>