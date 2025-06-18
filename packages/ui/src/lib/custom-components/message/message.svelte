<script lang="ts" module>
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const messageVariants = tv({
		base: 'message backdrop-blur-2xl bg-white/30 flex flex-col w-fit items-center gap-2 py-2 rounded-2xl [&_svg:not([class*="size-"])]:size-10 [&_svg]:pointer-events-none [&_svg]:shrink-0',
		variants: {
			variant: {
				default: 'max-w-[50%] ml-auto w-fit text-black font-medium',
				agent: 'max-w-[95%] mr-auto w-fit text-black font-medium',
			},
		},
		defaultVariants: {
			variant: 'default',
		},
	});

	export type MessageVariant = VariantProps<typeof messageVariants>['variant'];

	export type MessageProps = {
		variant?: MessageVariant;
		class?: string;
		href?: string;
		onclick?: ((event: MouseEvent) => void) | undefined;
		ref?: HTMLElement;
		finishRendering?: () => void;
		children?: any;
	};
</script>

<script lang="ts">
	let {
		ref = $bindable(),
		class: className,
		variant = 'default',
		children,
		...restProps
	}: MessageProps = $props();
</script>

<article bind:this={ref} class={cn(messageVariants({ variant }), className)} {...restProps}>
	{@render children?.()}
</article>

<style lang="postcss">
	@reference "tailwindcss";
	/* Apply solid background for Linux desktop app */
	:global(body.linux-app .message) {
		@apply bg-black/20 backdrop-blur-none blur-none;
	}
</style>
