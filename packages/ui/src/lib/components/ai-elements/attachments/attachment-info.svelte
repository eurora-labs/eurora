<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { getAttachmentItemContext, getAttachmentLabel } from './attachments-context.svelte.js';

	interface Props {
		class?: string;
		showMediaType?: boolean;
	}

	let { class: className, showMediaType = false, ...restProps }: Props = $props();

	let ctx = getAttachmentItemContext();
	let data = $derived(ctx.data);
	let variant = $derived(ctx.variant);
	let label = $derived(getAttachmentLabel(data));
</script>

{#if variant !== 'grid'}
	<div data-slot="attachment-info" class={cn('min-w-0 flex-1', className)} {...restProps}>
		<span class="block truncate">{label}</span>
		{#if showMediaType && data.mediaType}
			<span class="block truncate text-muted-foreground text-xs">
				{data.mediaType}
			</span>
		{/if}
	</div>
{/if}
