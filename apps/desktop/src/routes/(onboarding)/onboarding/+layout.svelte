<script lang="ts">
	let { children } = $props();
</script>

<main class="flex h-screen">
	<div class="w-2/3 p-8 overflow-y-auto">{@render children?.()}</div>
	<div class="w-1/3 h-full">static image</div>
</main>
