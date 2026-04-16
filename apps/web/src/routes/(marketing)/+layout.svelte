<script lang="ts">
	import MenuBar from '$lib/components/MenuBar.svelte';
	import Footer from '$lib/components/marketing/Footer.svelte';

	const { children } = $props();
</script>

<MenuBar />

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-16">
	{@render children?.()}
</div>

<Footer />
