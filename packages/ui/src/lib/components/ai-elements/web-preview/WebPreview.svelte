<script lang="ts">
	import { untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { WebPreviewContext, setWebPreviewContext } from './web-preview-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		defaultUrl?: string;
		onUrlChange?: (url: string) => void;
		children?: Snippet;
	}

	let {
		class: className,
		defaultUrl = '',
		onUrlChange,
		children,
		...restProps
	}: Props = $props();

	const context = untrack(
		() =>
			new WebPreviewContext({
				initialUrl: defaultUrl,
				onUrlChange: () => onUrlChange,
			}),
	);

	setWebPreviewContext(context);
</script>

<div
	data-slot="web-preview"
	class={cn('flex size-full flex-col rounded-lg border bg-card', className)}
	{...restProps}
>
	{@render children?.()}
</div>
