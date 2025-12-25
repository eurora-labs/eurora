<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="mx-auto max-h-[300px] max-w-[600px]">
	{@render children?.()}
</div>
