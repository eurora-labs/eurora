<script lang="ts">
	import { buttonVariants } from '$lib/components/button/index.js';
	import { cn } from '$lib/utils.js';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import { Calendar as CalendarPrimitive } from 'bits-ui';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: CalendarPrimitive.PrevButtonProps = $props();
</script>

{#snippet Fallback()}
	<ChevronLeftIcon class="size-4" />
{/snippet}

<CalendarPrimitive.PrevButton
	bind:ref
	class={cn(
		buttonVariants({ variant: 'outline' }),
		'size-7 bg-transparent p-0 opacity-50 hover:opacity-100',
		className,
	)}
	children={children || Fallback}
	{...restProps}
/>
