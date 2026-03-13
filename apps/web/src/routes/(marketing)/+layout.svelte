<script lang="ts">
	import MenuBar from '$lib/components/MenuBar.svelte';
	import Footer from '$lib/components/marketing/Footer.svelte';

	const { children } = $props();
</script>

<MenuBar />

{@render children?.()}

<Footer />
