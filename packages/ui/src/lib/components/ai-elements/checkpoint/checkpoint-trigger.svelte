<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import * as Tooltip from '$lib/components/tooltip/index.js';

	let {
		children,
		variant = 'ghost' as ButtonVariant,
		size = 'sm' as ButtonSize,
		tooltip,
		...restProps
	}: {
		children?: Snippet;
		variant?: ButtonVariant;
		size?: ButtonSize;
		tooltip?: string;
		[key: string]: any;
	} = $props();
</script>

{#if tooltip}
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<Button
					data-slot="checkpoint-trigger"
					{size}
					type="button"
					{variant}
					{...restProps}
					{...props}
				>
					{@render children?.()}
				</Button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content align="start" side="bottom">
			{tooltip}
		</Tooltip.Content>
	</Tooltip.Root>
{:else}
	<Button data-slot="checkpoint-trigger" {size} type="button" {variant} {...restProps}>
		{@render children?.()}
	</Button>
{/if}
