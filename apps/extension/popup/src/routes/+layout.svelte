<script lang="ts">
	import '../app.css';
	import '@eurora/ui/main.css';

	let { children } = $props();
</script>

<div class="mx-auto max-h-[300px] max-w-[600px]">
	{@render children?.()}
</div>
