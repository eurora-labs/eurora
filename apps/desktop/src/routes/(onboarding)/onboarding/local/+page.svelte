<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Ollama from './ollama.svelte';
	import ApiProvider from './api-provider.svelte';
	import { goto } from '$app/navigation';

	let status: 'pending' | 'finished' = $state('pending');

	async function finished() {
		status = 'finished';
		setTimeout(() => {
			goto('/onboarding/status');
		}, 1000);
	}
</script>

<div class="w-full h-screen mx-auto p-6 flex flex-col">
	{#if status === 'finished'}
		<div class="w-full h-screen mx-auto p-6 flex flex-col justify-center items-center">
			<h1 class="text-4xl font-bold mb-8">Connected, redirecting...</h1>
		</div>
	{:else}
		<h1 class="text-2xl font-bold mb-8">Third Party Configuration</h1>

		<div class="grid grid-cols-2 lg:grid-cols-2 gap-6 flex-1">
			<ApiProvider {finished} />
			<Ollama {finished} />
		</div>

		<div class="flex justify-between items-end mt-auto pt-8">
			<Button variant="outline" href="/onboarding">Back</Button>
		</div>
	{/if}
</div>
