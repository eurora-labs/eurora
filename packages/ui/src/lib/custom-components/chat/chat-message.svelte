<script lang="ts" module>
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';

	export const messageVariants = tv({
		base: 'message flex flex-col w-fit items-center gap-2 py-2 rounded-2xl [&_svg:not([class*="size-"])]:size-10 [&_svg]:pointer-events-none [&_svg]:shrink-0',
		variants: {
			variant: {
				default:
					'max-w-[50%] bg-black/20 ml-auto text-primary/80 font-medium dark:bg-white/20',
				assistant:
					'backdrop-blur-2xl bg-white/30 max-w-[95%] mr-auto text-primary/80 font-medium dark:bg-black/20',
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
