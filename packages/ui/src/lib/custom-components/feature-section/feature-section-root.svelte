<script lang="ts">
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		ref = $bindable(null),
		align = 'left' as 'left' | 'right',
		class: className,
		children,
		...restProps
	}: Omit<WithElementRef<HTMLAttributes<HTMLElement>>, 'align'> & {
		align?: 'left' | 'right';
	} = $props();
</script>

<section
	bind:this={ref}
	data-slot="feature-section"
	class={cn('w-full rounded-2xl bg-foreground/5 p-4 sm:p-6 lg:p-8', className)}
	{...restProps}
>
	<div
		class={cn(
			'flex w-full flex-row items-stretch',
			align === 'right' ? 'flex-row-reverse' : '',
		)}
	>
		{@render children?.()}
	</div>
</section>
