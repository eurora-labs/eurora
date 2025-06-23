<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Ollama from './ollama.svelte';
	import ApiProvider from './api-provider.svelte';
</script>

<div class="w-full h-screen mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Local AI Configuration</h1>

	<div class="grid grid-cols-2 lg:grid-cols-2 gap-6 flex-1">
		<ApiProvider />
		<Ollama />
	</div>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
	</div>
</div>
