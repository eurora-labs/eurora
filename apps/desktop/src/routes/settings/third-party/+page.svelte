<script lang="ts">
	import ApiProvider from '$lib/components/ApiProvider.svelte';
	import Ollama from '$lib/components/Ollama.svelte';

	// let status: 'pending' | 'finished' = $state('pending');

	async function finished() {
		// status = 'finished';
	}
</script>

<div class="w-full h-screen mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Third Party Configuration</h1>

	<div class="grid grid-cols-2 lg:grid-cols-2 gap-6 flex-1">
		<ApiProvider {finished} />
		<Ollama {finished} />
	</div>
</div>
