<script lang="ts">
	import type { WithElementRef, WithoutChildren } from 'bits-ui';
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { cn } from '@eurora/ui/utils.js';
	import { onMount } from 'svelte';

	let {
		ref = $bindable<HTMLTextAreaElement | null>(null),
		value = $bindable(''),
		minRows = $bindable(1),
		maxRows = $bindable(Infinity),
		class: className,
		...restProps
	}: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> & {
		minRows?: number;
		maxRows?: number;
	} = $props();

	let hiddenDiv: HTMLDivElement | undefined;
	let textareaRef: HTMLTextAreaElement | null = null;

	// Calculate and set the textarea height
	function adjustHeight() {
		if (!textareaRef || !hiddenDiv) return;

		// Copy the content to the hidden div
		const currentValue = typeof value === 'string' ? value : String(value || '');
		hiddenDiv.innerHTML = currentValue.replace(/\n/g, '<br />') + '&nbsp;'; // Add extra space to avoid shrinking on empty input

		// Calculate height with constraints
		const borderHeight = textareaRef.offsetHeight - textareaRef.clientHeight;
		const lineHeight = parseInt(getComputedStyle(textareaRef).lineHeight, 10) || 20;
		const minHeight = minRows * lineHeight;
		const maxHeight = maxRows === Infinity ? Infinity : maxRows * lineHeight;

		// Set the height of the textarea
		const newHeight = Math.min(
			Math.max(hiddenDiv.scrollHeight + borderHeight, minHeight),
			maxHeight
		);
		textareaRef.style.height = `${newHeight}px`;
	}

	// Watch for value changes and resize
	$effect(() => {
		value;
		setTimeout(adjustHeight, 0);
	});

	onMount(() => {
		if (!ref) return;
		// Assert that ref is an HTMLTextAreaElement since we're binding to a textarea element
		textareaRef = ref as HTMLTextAreaElement;

		if (!textareaRef || !hiddenDiv) return;

		// Set the initial styling of the hidden div to match the textarea
		const styles = window.getComputedStyle(textareaRef);
		[
			'fontFamily',
			'fontSize',
			'fontWeight',
			'letterSpacing',
			'paddingTop',
			'paddingRight',
			'paddingBottom',
			'paddingLeft',
			'boxSizing',
			'borderTopWidth',
			'borderRightWidth',
			'borderBottomWidth',
			'borderLeftWidth',
			'wordWrap',
			'lineHeight',
			'overflowWrap',
			'whiteSpace'
		].forEach((style) => {
			hiddenDiv!.style.setProperty(style, styles.getPropertyValue(style));
		});

		// Set specific styles needed for proper measurement
		hiddenDiv.style.position = 'absolute';
		hiddenDiv.style.top = '-9999px';
		hiddenDiv.style.left = '-9999px';
		hiddenDiv.style.width = `${textareaRef!.clientWidth}px`;
		hiddenDiv.style.height = 'auto';
		hiddenDiv.style.minHeight = 'auto';
		hiddenDiv.style.maxHeight = 'auto';
		hiddenDiv.style.visibility = 'hidden';

		// Initial resize
		adjustHeight();

		// Add resize observer to handle window/container resizing
		const resizeObserver = new ResizeObserver(() => {
			if (!textareaRef || !hiddenDiv) return;
			hiddenDiv.style.width = `${textareaRef.clientWidth}px`;
			adjustHeight();
		});

		if (textareaRef) {
			resizeObserver.observe(textareaRef);
		}

		return () => {
			resizeObserver.disconnect();
		};
	});
</script>

<div bind:this={hiddenDiv} aria-hidden="true"></div>

<textarea
	bind:this={ref}
	bind:value
	class={cn(
		'flex w-full resize-none overflow-hidden rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
		className
	)}
	on:input={adjustHeight}
	{...restProps}
></textarea>
