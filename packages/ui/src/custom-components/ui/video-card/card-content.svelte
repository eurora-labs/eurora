<script lang="ts">
	import type { WithElementRef } from 'bits-ui';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		mp4Src = '',
		webmSrc = '',
		alignment = 'left',
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		mp4Src?: string;
		webmSrc?: string;
		alignment?: 'left' | 'right';
	} = $props();
</script>

<div
	bind:this={ref}
	class={cn(
		'flex gap-0 overflow-hidden p-0',
		alignment === 'right' ? 'flex-row-reverse' : 'flex-row',
		className
	)}
	{...restProps}
>
	<div class="h-full w-3/5 overflow-hidden rounded-[104px]">
		<video class="h-full w-full object-cover" controls>
			{#if webmSrc}
				<source src={webmSrc} type="video/webm" />
			{/if}
			{#if mp4Src}
				<source src={mp4Src} type="video/mp4" />
			{/if}
			Your browser does not support the video tag.
		</video>
	</div>
	<div class="flex w-2/5 items-center justify-center px-8">
		<div class="flex w-full flex-col space-y-4">
			{@render children?.()}
		</div>
	</div>
</div>
