<script lang="ts">
	import '../app.pcss';
	import '@eurora/ui/main.css';
	// import Terminal from "@lucide/svelte/icons/terminal";
	let { children } = $props();
</script>

<div class="mx-0 w-full h-full">
	{@render children?.()}
</div>
