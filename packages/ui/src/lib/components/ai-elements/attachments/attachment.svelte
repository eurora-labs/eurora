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
			'flex h-9 cursor-pointer select-none items-center gap-2',
			'rounded-md bg-primary px-3',
			'text-sm font-medium text-primary-foreground shadow-xs transition-all duration-150',
			'hover:bg-primary/90 hover:shadow-md',
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
