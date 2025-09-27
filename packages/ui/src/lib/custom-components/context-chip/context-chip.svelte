<script lang="ts" module>
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const contextChipVariants = tv({
		base: 'context-chip inline-flex w-fit items-center gap-2 my-2 p-2 bg-transparent rounded-2xl backdrop-blur-sm text-black/70 [&_svg:not([class*="size-"])]:size-10 [&_svg]:pointer-events-none [&_svg]:shrink-0',
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

	export type ContextChipProps = {
		variant?: ContextChipVariant;
		class?: string;
		href?: string;
		onclick?: ((event: MouseEvent) => void) | undefined;
		ref?: HTMLElement;
	};
</script>

<script lang="ts">
	let {
		class: className,
		variant = 'default',
		ref = $bindable(),
		href = undefined,
		onclick = undefined,
		children,
		...restProps
	}: ContextChipProps & {
		children?: any;
		[key: string]: any;
	} = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		class={cn(contextChipVariants({ variant }), className)}
		{href}
		{onclick}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else if onclick}
	<button
		bind:this={ref}
		class={cn(contextChipVariants({ variant }), className)}
		{onclick}
		type="button"
		{...restProps}
	>
		{@render children?.()}
	</button>
{:else}
	<span bind:this={ref} class={cn(contextChipVariants({ variant }), className)} {...restProps}>
		{@render children?.()}
	</span>
{/if}

<style lang="postcss">
	/*@reference 'tailwindcss';*/
	/* Apply solid background for Linux desktop app */
	:global(body.linux-app .context-chip) {
		@apply bg-black/20 backdrop-blur-none blur-none;
	}
</style>
