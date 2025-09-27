<script lang="ts">
	import type { WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

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
		/* Desktop: row layout with responsive height */
		'md:flex-row md:gap-0 md:h-[70vh] md:max-h-[1000px] md:min-h-[400px]',
		className,
	)}
	{...restProps}
>
	<!-- Video content -->
	<div
		class={cn(
			'order-2 h-[250px] overflow-hidden rounded-[24px] md:h-full md:w-3/5 md:rounded-[104px]',
			alignment === 'right' ? 'md:order-2' : 'md:order-1',
		)}
	>
		<video class="h-full w-full object-cover" controls>
			{#if webmSrc}
				<source src={webmSrc} type="video/webm" />
			{/if}
			{#if mp4Src}
				<source src={mp4Src} type="video/mp4" />
			{/if}
			<track kind="captions" src="" srclang="en" label="English" />
			Your browser does not support the video tag.
		</video>
	</div>

	<!-- Text content -->
	<div
		class={cn(
			'order-1 flex items-center justify-center px-4 md:w-2/5 md:px-8',
			alignment === 'right' ? 'md:order-1' : 'md:order-2',
		)}
	>
		<div class="flex w-full flex-col space-y-4">
			{@render children?.()}
		</div>
	</div>
</div>

<style lang="postcss">
	/*@reference 'tailwindcss';*/
</style>
