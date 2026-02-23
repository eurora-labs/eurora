<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { getWebPreviewContext } from './web-preview-context.svelte.js';

	interface Props extends HTMLInputAttributes {
		value?: string;
	}

	let { class: className, value: valueProp, ...restProps }: Props = $props();

	let context = getWebPreviewContext();
	let inputValue = $state(valueProp ?? context.url);

	$effect(() => {
		if (valueProp !== undefined) {
			inputValue = valueProp;
		}
	});

	$effect(() => {
		inputValue = context.url;
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			context.url = inputValue;
		}
	}

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		inputValue = target.value;
	}
</script>

<input
	data-slot="web-preview-url"
	class={cn(
		'h-8 flex-1 rounded-md border border-input bg-transparent px-3 text-sm shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]',
		className,
	)}
	placeholder="Enter URL..."
	value={inputValue}
	oninput={handleInput}
	onkeydown={handleKeydown}
	{...restProps}
/>
