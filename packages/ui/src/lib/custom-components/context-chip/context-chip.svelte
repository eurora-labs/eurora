<script lang="ts" module>
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const contextChipVariants = tv({
		base: 'inline-flex w-fit items-center gap-2 mx-2 p-2 text-[40px] leading-[40px] rounded-2xl backdrop-blur-md text-black/70 [&_svg:not([class*="size-"])]:size-10 [&_svg]:pointer-events-none [&_svg]:shrink-0',
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
		ref?: HTMLElement | null;
	};
</script>

<script lang="ts">
	let {
		class: className,
		variant = 'default',
		ref = $bindable(null),
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
		class={cn(contextChipVariants({ variant }), className, 'context-chip')}
		{href}
		{onclick}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else if onclick}
	<button
		bind:this={ref}
		class={cn(contextChipVariants({ variant }), className, 'context-chip')}
		{onclick}
		type="button"
		{...restProps}
	>
		{@render children?.()}
	</button>
{:else}
	<span
		bind:this={ref}
		class={cn(contextChipVariants({ variant }), className, 'context-chip')}
		{...restProps}
	>
		{@render children?.()}
	</span>
{/if}

<style lang="postcss">
	:global(.context-chip) {
		display: inline-flex;
		border-radius: 16px;
		backdrop-filter: blur(6px);
		-webkit-backdrop-filter: blur(6px);
		background-color: transparent;
		color: rgba(0, 0, 0, 1);
	}

	/* Apply solid background for Linux desktop app */
	:global(body.linux-app .context-chip) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: rgba(0, 0, 0, 0.2);
	}
</style>
