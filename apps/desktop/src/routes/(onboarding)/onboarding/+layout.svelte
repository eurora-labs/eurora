<script lang="ts">
	let { children } = $props();
</script>

<div class="flex h-screen w-screen">
	<div class="w-2/3 p-8 h-full">{@render children?.()}</div>
	<div class="w-1/3 h-full">static image</div>
</div>
