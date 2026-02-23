<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getWebPreviewContext } from './web-preview-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLIFrameElement> {
		src?: string;
		loading?: Snippet;
	}

	let { class: className, src, loading, ...restProps }: Props = $props();

	let context = getWebPreviewContext();
	let iframeSrc = $derived(src ?? context.url || undefined);
</script>

<div data-slot="web-preview-body" class="flex-1">
	<iframe
		class={cn('size-full', className)}
		sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
		src={iframeSrc}
		title="Preview"
		{...restProps}
	></iframe>
	{@render loading?.()}
</div>
