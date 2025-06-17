<script lang="ts" module>
	export const conversationItemVariants = tv({
		base: 'conversation-item flex w-fit items-center gap-2 py-2 px-4 rounded-2xl [&_svg:not([class*="size-"])]:size-10 [&_svg]:pointer-events-none [&_svg]:shrink-0',
		variants: {
			variant: {
				default: 'justify-self-end max-w-[50%] w-fit bg-white/20 text-black font-medium',
				agent: 'justify-self-start w-fit bg-white/40 text-black font-medium',
			},
		},
		defaultVariants: {
			variant: 'default',
		},
	});

	export type ConversationItemVariant = VariantProps<typeof conversationItemVariants>['variant'];

	export type ConversationItemProps = {
		variant?: ConversationItemVariant;
		class?: string;
		href?: string;
		onclick?: ((event: MouseEvent) => void) | undefined;
		ref?: HTMLElement;
		finishRendering?: () => void;
		children?: any;
	};
</script>

<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { type VariantProps, tv } from 'tailwind-variants';

	let {
		ref = $bindable(),
		class: className,
		variant = 'default',
		children,
		...restProps
	}: ConversationItemProps = $props();
</script>

<article
	bind:this={ref}
	class={cn(conversationItemVariants({ variant }), className)}
	{...restProps}
>
	{@render children?.()}
</article>

<style lang="postcss">
	@reference "tailwindcss";
	/* Apply solid background for Linux desktop app */
	:global(body.linux-app .conversation-item) {
		@apply bg-black/20 backdrop-blur-none blur-none;
	}
</style>
