<script lang="ts">
	import '../app.pcss';

	// import Terminal from "@lucide/svelte/icons/terminal";
	let { children } = $props();
</script>

<div class="min-h-screen container mx-auto"> 

	{@render children?.()}
</div>
