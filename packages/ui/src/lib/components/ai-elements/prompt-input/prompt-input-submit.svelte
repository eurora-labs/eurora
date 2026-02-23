<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonVariant, type ButtonSize } from '$lib/components/button/index.js';
	import { Spinner } from '$lib/components/spinner/index.js';
	import CornerDownLeftIcon from '@lucide/svelte/icons/corner-down-left';
	import SquareIcon from '@lucide/svelte/icons/square';
	import XIcon from '@lucide/svelte/icons/x';
	import type { ChatStatus } from './prompt-input-context.svelte.js';

	interface Props {
		class?: string;
		variant?: ButtonVariant;
		size?: ButtonSize;
		disabled?: boolean;
		status?: ChatStatus;
		onStop?: () => void;
		onclick?: (e: MouseEvent & { currentTarget: EventTarget & HTMLElement }) => void;
		children?: Snippet;
	}

	let {
		class: className,
		variant = 'default',
		size = 'icon-sm',
		status = undefined,
		onStop = undefined,
		onclick = undefined,
		disabled = undefined,
		children,
	}: Props = $props();

	const isGenerating = $derived(status === 'submitted' || status === 'streaming');

	function handleClick(e: MouseEvent & { currentTarget: EventTarget & HTMLElement }) {
		if (isGenerating && onStop) {
			e.preventDefault();
			onStop();
			return;
		}
		onclick?.(e);
	}
</script>

<Button
	data-slot="prompt-input-submit"
	aria-label={isGenerating ? 'Stop' : 'Submit'}
	class={cn(className)}
	onclick={handleClick}
	{disabled}
	{size}
	type={isGenerating && onStop ? 'button' : 'submit'}
	{variant}
>
	{#if children}
		{@render children()}
	{:else if status === 'submitted'}
		<Spinner />
	{:else if status === 'streaming'}
		<SquareIcon class="size-4" />
	{:else if status === 'error'}
		<XIcon class="size-4" />
	{:else}
		<CornerDownLeftIcon class="size-4" />
	{/if}
</Button>
