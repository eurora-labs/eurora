<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import { getAttachmentItemContext } from './attachments-context.svelte.js';
	import XIcon from '@lucide/svelte/icons/x';
	import type { Snippet } from 'svelte';

	interface Props {
		class?: string;
		label?: string;
		children?: Snippet;
	}

	let { class: className, label = 'Remove', children, ...restProps }: Props = $props();

	let ctx = getAttachmentItemContext();
	let onRemove = $derived(ctx.onRemove);
	let variant = $derived(ctx.variant);

	function handleClick(e: MouseEvent) {
		e.stopPropagation();
		onRemove?.();
	}
</script>

{#if onRemove}
	<Button
		data-slot="attachment-remove"
		aria-label={label}
		class={cn(
			variant === 'grid' && [
				'absolute top-2 right-2 size-6 rounded-full p-0',
				'bg-background/80 backdrop-blur-sm',
				'opacity-0 transition-opacity group-hover:opacity-100',
				'hover:bg-background',
				'[&>svg]:size-3',
			],
			variant === 'inline' && [
				'size-5 rounded p-0',
				'opacity-0 transition-opacity group-hover:opacity-100',
				'[&>svg]:size-2.5',
			],
			variant === 'list' && ['size-8 shrink-0 rounded p-0', '[&>svg]:size-4'],
			className,
		)}
		onclick={handleClick}
		type="button"
		variant="ghost"
		{...restProps}
	>
		{#if children}
			{@render children()}
		{:else}
			<XIcon />
		{/if}
		<span class="sr-only">{label}</span>
	</Button>
{/if}
