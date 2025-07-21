<script lang="ts">
	let { children } = $props();
</script>

<main class="container mx-auto">
	{@render children?.()}
</main>
