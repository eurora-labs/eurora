<script lang="ts">
	import { type Variant, contextChipVariants } from './index.js';
	import { cn } from '@eurora/ui/utils.js';

	let ref: HTMLElement | undefined = undefined;
	let className: string | undefined | null = undefined;
	export let href: string | undefined = undefined;
	export let variant: Variant = 'default';
	export let onClick: ((event: MouseEvent) => void) | undefined = undefined;
	export { className as class };
</script>

<svelte:element
	this={href ? 'a' : 'span'}
	{href}
	role={onClick ? 'button' : undefined}
	on:click={onClick}
	bind:this={ref}
	class={cn(contextChipVariants({ variant, className }), 'context-chip')}
	{...$$restProps}
>
	<slot />
</svelte:element>

<style lang="postcss">
	:global(.context-chip) {
		@apply w-fit items-center gap-2 text-[40px] leading-[40px] text-white;
		@apply mx-2 p-2;
		color: rgba(0, 0, 0, 0.7);
		background: transparent;
		border-radius: 16px;
		display: inline-block;
		backdrop-filter: blur(6px);
		-webkit-backdrop-filter: blur(6px);
	}
</style>
