<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import AlertTriangleIcon from '@lucide/svelte/icons/triangle-alert';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<div
	data-slot="stack-trace-error"
	class={cn('flex flex-1 items-center gap-2 overflow-hidden', className)}
	{...rest}
>
	<AlertTriangleIcon class="size-4 shrink-0 text-destructive" />
	{@render children?.()}
</div>
