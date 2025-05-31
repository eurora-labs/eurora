<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.pcss';
	// import Terminal from "@lucide/svelte/icons/terminal";
	let { children } = $props();
</script>

<div class="container mx-auto min-h-screen">
	{@render children?.()}
</div>
