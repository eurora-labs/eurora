<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { AlertDescription } from '$lib/components/alert/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<AlertDescription
	data-slot="confirmation-title"
	class={cn('inline', className)}
	{...restProps}
>
	{@render children?.()}
</AlertDescription>
