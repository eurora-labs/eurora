<script lang="ts">
	import type { Snippet, Component } from 'svelte';
	import { cn } from '$lib/utils.js';
	import DotIcon from '@lucide/svelte/icons/dot';

	type Status = 'complete' | 'active' | 'pending';

	const stepStatusStyles: Record<Status, string> = {
		active: 'text-foreground',
		complete: 'text-muted-foreground',
		pending: 'text-muted-foreground/50',
	};

	interface Props {
		icon?: Component<{ class?: string }>;
		label?: Snippet;
		description?: Snippet;
		status?: Status;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		icon: Icon = DotIcon,
		label,
		description,
		status = 'complete',
		class: className,
		children,
		...rest
	}: Props = $props();
</script>

<div
	data-slot="chain-of-thought-step"
	class={cn(
		'flex gap-2 text-sm',
		stepStatusStyles[status],
		'fade-in-0 slide-in-from-top-2 animate-in',
		className,
	)}
	{...rest}
>
	<div class="relative mt-0.5">
		<Icon class="size-4" />
		<div class="bg-border absolute top-7 bottom-0 left-1/2 -mx-px w-px"></div>
	</div>
	<div class="flex-1 space-y-2 overflow-hidden">
		{#if label}
			<div>{@render label()}</div>
		{/if}
		{#if description}
			<div class="text-muted-foreground text-xs">{@render description()}</div>
		{/if}
		{@render children?.()}
	</div>
</div>
