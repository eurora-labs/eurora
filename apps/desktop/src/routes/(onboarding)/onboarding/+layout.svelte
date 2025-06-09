<script lang="ts">
	let { children } = $props();
</script>

<div class="flex h-screen w-screen">
	<div class="h-full w-2/3 p-8">{@render children?.()}</div>
	<div class="h-full w-1/3">static image</div>
</div>
