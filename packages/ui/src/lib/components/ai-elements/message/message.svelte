<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import type { MessageRole } from './message-context.svelte.js';

	let {
		class: className,
		children,
		from,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
		from: MessageRole;
	} = $props();
</script>

<div
	data-slot="message"
	class={cn(
		'group flex w-full max-w-[95%] flex-col gap-2',
		from === 'user' ? 'is-user ml-auto justify-end' : 'is-assistant',
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
