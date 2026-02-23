<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import * as Tooltip from '$lib/components/tooltip/index.js';

	let {
		tooltip,
		label,
		children,
		class: className,
		size = 'icon-sm' as ButtonSize,
		variant = 'ghost' as ButtonVariant,
		...restProps
	}: {
		tooltip?: string;
		label?: string;
		children?: Snippet;
		class?: string;
		size?: ButtonSize;
		variant?: ButtonVariant;
		[key: string]: any;
	} = $props();
</script>

{#snippet button(extraProps?: Record<string, any>)}
	<Button
		data-slot="message-action"
		class={cn(className)}
		{size}
		type="button"
		{variant}
		{...restProps}
		{...extraProps}
	>
		{@render children?.()}
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
