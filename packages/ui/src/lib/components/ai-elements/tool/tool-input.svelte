<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	// TODO: Replace with CodeBlock when implemented
	let {
		class: className,
		input,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		input: unknown;
	} = $props();

	let formatted = $derived(JSON.stringify(input, null, 2));
</script>

<div data-slot="tool-input" class={cn('space-y-2 overflow-hidden', className)} {...restProps}>
	<h4 class="font-medium text-muted-foreground text-xs uppercase tracking-wide">
		Parameters
	</h4>
	<div class="rounded-md bg-muted/50">
		<!-- TODO: Replace with CodeBlock when implemented -->
		<pre class="overflow-x-auto p-3 text-xs">{formatted}</pre>
	</div>
</div>
