<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getStackTraceContext } from './stack-trace-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();

	let ctx = getStackTraceContext();

	function handleClick() {
		ctx.isOpen = !ctx.isOpen;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			ctx.isOpen = !ctx.isOpen;
		}
	}
</script>

<div
	data-slot="stack-trace-header"
	class={cn(
		'flex w-full cursor-pointer items-center gap-3 p-3 text-left transition-colors hover:bg-muted/50',
		className,
	)}
	onclick={handleClick}
	onkeydown={handleKeydown}
	role="button"
	tabindex="0"
	{...rest}
>
	{@render children?.()}
</div>
