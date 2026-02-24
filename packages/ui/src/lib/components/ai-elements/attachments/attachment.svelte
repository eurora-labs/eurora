<script lang="ts">
	import { cn } from '$lib/utils.js';
	import {
		getAttachmentsContext,
		getMediaCategory,
		setAttachmentItemContext,
		type AttachmentData,
	} from './attachments-context.svelte.js';
	import { watch } from 'runed';

	interface Props {
		class?: string;
		data: AttachmentData;
		onRemove?: () => void;
		children?: import('svelte').Snippet;
	}

	let { class: className, data, onRemove, children, ...restProps }: Props = $props();

	let attachmentsCtx = getAttachmentsContext();
	let variant = $derived(attachmentsCtx.variant);

	let itemCtx = setAttachmentItemContext(data, variant, onRemove);

	watch(
		() => [data, variant, onRemove] as const,
		([d, v, r]) => {
			itemCtx.data = d;
			itemCtx.variant = v;
			itemCtx.onRemove = r;
			itemCtx.mediaCategory = getMediaCategory(d);
		},
	);
</script>

<div
	data-slot="attachment"
	class={cn(
		'group relative',
		variant === 'grid' && 'size-24 overflow-hidden rounded-lg',
		variant === 'inline' && [
			'flex h-8 cursor-pointer select-none items-center gap-1.5',
			'rounded-md border border-border px-1.5',
			'font-medium transition-all',
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
