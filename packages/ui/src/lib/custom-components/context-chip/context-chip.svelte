<script lang="ts" module>
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const contextChipVariants = tv({
		base: 'inline-block w-fit items-center gap-2 mx-2 p-2 text-[40px] leading-[40px] rounded-2xl backdrop-blur-md text-black/70',
		variants: {
			variant: {
				default: 'bg-white/30',
				primary: 'bg-primary/30 text-primary-foreground',
				secondary: 'bg-secondary/30 text-secondary-foreground',
				destructive: 'bg-destructive/30 text-destructive-foreground',
				outline: 'border border-input bg-transparent',
			},
		},
		defaultVariants: {
			variant: 'default',
		},
	});

	export type ContextChipVariant = VariantProps<typeof contextChipVariants>['variant'];

	export type ContextChipProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ContextChipVariant;
		};
</script>

<script lang="ts">
	let ref: HTMLElement | undefined = undefined;
	let className: string | undefined | null = undefined;
	export let href: string | undefined = undefined;
	export let variant: ContextChipVariant = 'default';
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
		display: inline-block;
		border-radius: 16px;
		backdrop-filter: blur(6px);
		-webkit-backdrop-filter: blur(6px);
		background-color: transparent;
		/* @apply w-fit items-center gap-2 text-[40px] leading-[40px] text-white; */
		/* @apply mx-2 p-2; */
		color: rgba(0, 0, 0, 1);
	}

	/* Apply solid background for Linux desktop app */
	:global(body.linux-app .context-chip) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: rgba(0, 0, 0, 0.2);
	}
</style>
