<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/tooltip/index.js';

	let {
		variant = 'ghost',
		class: className,
		size = 'icon-sm',
		tooltip = undefined,
		children,
		...restProps
	}: ButtonProps & {
		tooltip?:
			| string
			| {
					content: string;
					shortcut?: string;
					side?: 'top' | 'bottom' | 'left' | 'right';
				};
		children?: Snippet;
	} = $props();

	const tooltipContent = $derived(
		typeof tooltip === 'string' ? tooltip : tooltip?.content,
	);
	const shortcut = $derived(typeof tooltip === 'string' ? undefined : tooltip?.shortcut);
	const side = $derived(typeof tooltip === 'string' ? 'top' : (tooltip?.side ?? 'top'));
</script>

{#if tooltip}
	<Tooltip>
		<TooltipTrigger>
			{#snippet child({ props: triggerProps })}
				<Button
					data-slot="prompt-input-button"
					class={cn(className)}
					{size}
					type="button"
					{variant}
					{...triggerProps}
					{...restProps}
				>
					{@render children?.()}
				</Button>
			{/snippet}
		</TooltipTrigger>
		<TooltipContent {side}>
			{tooltipContent}
			{#if shortcut}
				<span class="ml-2 text-muted-foreground">{shortcut}</span>
			{/if}
		</TooltipContent>
	</Tooltip>
{:else}
	<Button
		data-slot="prompt-input-button"
		class={cn(className)}
		{size}
		type="button"
		{variant}
		{...restProps}
	>
		{@render children?.()}
	</Button>
{/if}
