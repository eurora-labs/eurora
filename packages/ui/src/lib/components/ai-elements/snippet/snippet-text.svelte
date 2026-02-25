<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { InputGroupText } from '$lib/components/input-group/index.js';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();
</script>

<InputGroupText
	data-slot="snippet-text"
	class={cn('pl-2 font-normal text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</InputGroupText>
