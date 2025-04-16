<script lang="ts">
	import '../../../app.pcss';
	let { children } = $props();
</script>

<main class="flex-1 overflow-hidden bg-transparent">
	{@render children?.()}
</main>
