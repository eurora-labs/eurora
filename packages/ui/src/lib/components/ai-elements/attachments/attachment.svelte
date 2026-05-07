<script lang="ts">
	import { cn } from '$lib/utils.js';
	import {
		getAttachmentsContext,
		setAttachmentItemContext,
		type AttachmentData,
	} from './attachments-context.svelte.js';
	import type { Snippet } from 'svelte';

	interface Props {
		class?: string;
		data: AttachmentData;
		onRemove?: () => void;
		children?: Snippet;
	}

	let { class: className, data, onRemove, children, ...restProps }: Props = $props();

	const attachmentsCtx = getAttachmentsContext();
	const variant = $derived(attachmentsCtx.variant);

	setAttachmentItemContext({
		data: () => data,
		variant: () => variant,
		onRemove: () => onRemove,
	});
</script>

<div
	data-slot="attachment"
	class={cn(
		'group relative',
		variant === 'grid' && 'size-24 overflow-hidden rounded-lg',
		variant === 'inline' && [
			'flex h-8 cursor-pointer select-none items-center gap-1.5',
			'rounded-md border border-border px-1.5',
			'font-medium text-sm transition-all',
			'hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50',
		],
		variant === 'list' && [
			'flex w-full items-center gap-3 rounded-lg border p-3',
			'hover:bg-accent/50',
		],
		className,
	)}
	{...restProps}
>
	{@render children?.()}
</div>
