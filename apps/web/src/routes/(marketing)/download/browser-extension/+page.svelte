<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
</script>

<svelte:head>
	<title>Browser Extension — Eurora Labs</title>
	<meta
		name="description"
		content="Install the Eurora browser extension for Chrome, Firefox, Edge, or Safari. Give your AI assistant full context from the web pages you browse."
	/>
</svelte:head>

<div class="flex min-h-screen flex-col items-center justify-center gap-6 p-8">
	<article class="prose max-w-none">
		<h1 class="text-4xl font-bold">Download Browser Extension</h1>
		<p>
			The browser extension gives Eurora context from the pages you read, so you can ask
			questions without copying or pasting.
		</p>
	</article>
	<Button onclick={() => goto('browser-extension/redirect')}>Download</Button>
</div>
