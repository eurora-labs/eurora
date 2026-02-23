<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import Paperclip from '@lucide/svelte/icons/paperclip';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();
</script>

<span
	data-slot="queue-item-file"
	class={cn(
		'flex items-center gap-1 rounded border bg-muted px-2 py-1 text-xs',
		className,
	)}
	{...restProps}
>
	<Paperclip size={12} />
	<span class="max-w-[100px] truncate">{@render children?.()}</span>
</span>
