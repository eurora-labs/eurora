<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import {
		InputGroupButton,
		type InputGroupButtonSize,
	} from '$lib/components/input-group/index.js';
	import { Tooltip, TooltipTrigger, TooltipContent } from '$lib/components/tooltip/index.js';
	import type { ButtonVariant } from '$lib/components/button/index.js';

	let {
		variant = 'ghost',
		class: className,
		size = 'icon-sm' as InputGroupButtonSize,
		tooltip = undefined,
		children,
		...restProps
	}: {
		variant?: ButtonVariant;
		class?: string;
		size?: InputGroupButtonSize;
		tooltip?:
			| string
			| {
					content: string;
					shortcut?: string;
					side?: 'top' | 'bottom' | 'left' | 'right';
			  };
		children?: Snippet;
		[key: string]: unknown;
	} = $props();

	const tooltipContent = $derived(typeof tooltip === 'string' ? tooltip : tooltip?.content);
	const shortcut = $derived(typeof tooltip === 'string' ? undefined : tooltip?.shortcut);
	const side = $derived(typeof tooltip === 'string' ? 'top' : (tooltip?.side ?? 'top'));
</script>

{#if tooltip}
	<Tooltip>
		<TooltipTrigger>
			{#snippet child({ props: triggerProps })}
				<InputGroupButton
					data-slot="prompt-input-button"
					class={cn(className)}
					{size}
					{variant}
					{...triggerProps}
					{...restProps}
				>
					{@render children?.()}
				</InputGroupButton>
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
	<InputGroupButton
		data-slot="prompt-input-button"
		class={cn(className)}
		{size}
		{variant}
		{...restProps}
	>
		{@render children?.()}
	</InputGroupButton>
{/if}
