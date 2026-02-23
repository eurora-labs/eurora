<script lang="ts">
	import type { Snippet, Component } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import * as Tooltip from '$lib/components/tooltip/index.js';

	let {
		tooltip,
		label,
		icon: Icon,
		children,
		class: className,
		size = 'sm' as ButtonSize,
		variant = 'ghost' as ButtonVariant,
		...restProps
	}: {
		tooltip?: string;
		label?: string;
		icon?: Component<{ class?: string }>;
		children?: Snippet;
		class?: string;
		size?: ButtonSize;
		variant?: ButtonVariant;
		[key: string]: any;
	} = $props();
</script>

{#snippet button(extraProps?: Record<string, any>)}
	<Button
		data-slot="artifact-action"
		class={cn('size-8 p-0 text-muted-foreground hover:text-foreground', className)}
		{size}
		type="button"
		{variant}
		{...restProps}
		{...extraProps}
	>
		{#if Icon}
			<Icon class="size-4" />
		{:else}
			{@render children?.()}
		{/if}
		<span class="sr-only">{label || tooltip}</span>
	</Button>
{/snippet}

{#if tooltip}
	<Tooltip.Root>
		<Tooltip.Trigger asChild>
			{#snippet child({ props })}
				{@render button(props)}
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content>
			<p>{tooltip}</p>
		</Tooltip.Content>
	</Tooltip.Root>
{:else}
	{@render button()}
{/if}
