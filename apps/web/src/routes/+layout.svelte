<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.pcss';
	// import Terminal from "@lucide/svelte/icons/terminal";
	let { children } = $props();
</script>

<div class="mx-0 w-full h-full">
	{@render children?.()}
</div>
