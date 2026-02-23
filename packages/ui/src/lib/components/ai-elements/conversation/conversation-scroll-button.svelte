<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import { getStickToBottomContext } from './conversation-context.svelte.js';
	import { fly } from 'svelte/transition';
	import { backOut } from 'svelte/easing';

	let { class: className, onclick, ...restProps }: ButtonProps = $props();

	const context = getStickToBottomContext();

	const handleScrollToBottom = (event: MouseEvent) => {
		context.scrollToBottom();
		if (onclick) {
			onclick(
				event as MouseEvent & {
					currentTarget: EventTarget & HTMLButtonElement;
				},
			);
		}
	};
</script>

{#if !context.isAtBottom}
	<div
		in:fly={{
			duration: 300,
			y: 10,
			easing: backOut,
		}}
		out:fly={{
			duration: 200,
			y: 10,
			easing: backOut,
		}}
		class="absolute bottom-4 left-[50%] translate-x-[-50%]"
	>
		<Button
			data-slot="conversation-scroll-button"
			class={cn(
				'bg-background/80 border-border/50 hover:bg-background/90 rounded-full shadow-lg backdrop-blur-sm hover:shadow-xl',
				className,
			)}
			onclick={handleScrollToBottom}
			size="icon"
			type="button"
			variant="outline"
			{...restProps}
		>
			<ArrowDown class="size-4" />
		</Button>
	</div>
{/if}
