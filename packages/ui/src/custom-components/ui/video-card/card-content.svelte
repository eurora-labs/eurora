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
		/* Mobile: column layout with text at top, video at bottom */
		'flex-col gap-6',
		/* Desktop: row layout */
		alignment === 'right'
			? 'md:flex-row-reverse md:gap-0'
			: 'md:flex-row md:gap-0',
		className
	)}
	{...restProps}
>
	<!-- Video content - first in DOM order but appears second (bottom) on mobile -->
	<div class="w-full md:w-3/5 overflow-hidden rounded-[24px] md:rounded-[104px] order-2 md:order-1 h-[250px] md:h-full">
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
	
	<!-- Text content - second in DOM order but appears first (top) on mobile -->
	<div class="flex w-full md:w-2/5 items-start md:items-center justify-center px-4 md:px-8 order-1 md:order-2">
		<div class="flex w-full flex-col space-y-4">
			{@render children?.()}
		</div>
	</div>
</div>
