<script lang="ts">
	import { goto } from '$app/navigation';
	import ApiProvider from '$lib/components/ApiProvider.svelte';
	import Ollama from '$lib/components/Ollama.svelte';
	import { Button } from '@eurora/ui/components/button/index';

	async function finished() {
		goto('browser-extension');
	}
</script>

<div class="w-full h-full mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Third Party Configuration</h1>

	<div class="grid grid-cols-2 lg:grid-cols-2 gap-6 flex-1">
		<ApiProvider {finished} />
		<Ollama {finished} />
	</div>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
	</div>
</div>
