<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { tv } from 'tailwind-variants';
	import { setAttachmentsContext, type AttachmentVariant } from './attachments-context.svelte.js';
	import { watch } from 'runed';

	interface Props {
		class?: string;
		variant?: AttachmentVariant;
		children?: import('svelte').Snippet;
	}

	let { class: className, variant = 'grid', children, ...restProps }: Props = $props();

	const attachmentsVariants = tv({
		base: 'flex items-start',
		variants: {
			variant: {
				grid: 'ml-auto w-fit flex-wrap gap-2',
				inline: 'flex-wrap gap-2',
				list: 'flex-col gap-2',
			},
		},
		defaultVariants: {
			variant: 'grid',
		},
	});

	let contextInstance = setAttachmentsContext(variant);

	watch(
		() => variant,
		() => {
			contextInstance.variant = variant;
		},
	);
</script>

<div
	data-slot="attachments"
	class={cn(attachmentsVariants({ variant }), className)}
	{...restProps}
>
	{@render children?.()}
</div>
