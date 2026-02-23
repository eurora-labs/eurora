<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import type { GenderValue } from './voice-selector-context.svelte.js';
	import MarsIcon from '@lucide/svelte/icons/mars';
	import VenusIcon from '@lucide/svelte/icons/venus';
	import TransgenderIcon from '@lucide/svelte/icons/transgender';
	import MarsStrokeIcon from '@lucide/svelte/icons/mars-stroke';
	import CircleSmallIcon from '@lucide/svelte/icons/circle-small';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		value?: GenderValue;
		children?: Snippet;
	}

	let { class: className, value, children, ...restProps }: Props = $props();
</script>

<span
	data-slot="voice-selector-gender"
	class={cn('text-muted-foreground text-xs', className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else if value === 'male'}
		<MarsIcon class="size-4" />
	{:else if value === 'female'}
		<VenusIcon class="size-4" />
	{:else if value === 'transgender'}
		<TransgenderIcon class="size-4" />
	{:else if value === 'androgyne'}
		<MarsStrokeIcon class="size-4" />
	{:else if value === 'non-binary' || value === 'intersex'}
		<CircleSmallIcon class="size-4" />
	{:else}
		<CircleSmallIcon class="size-4" />
	{/if}
</span>
