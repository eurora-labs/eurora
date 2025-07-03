<script lang="ts">
	import '../app.pcss';
	import '@eurora/ui/main.css';
	let { children } = $props();
</script>

<main class="h-screen p-0 m-0">
	{@render children?.()}
</main>
