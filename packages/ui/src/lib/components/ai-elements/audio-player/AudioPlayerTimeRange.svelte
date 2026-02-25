<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { ButtonGroupText } from '$lib/components/button-group/index.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<ButtonGroupText class="bg-transparent">
	{#snippet child({ props })}
		<media-time-range
			data-slot="audio-player-time-range"
			{...props}
			class={cn(props.class as string, className)}
			{...restProps}
		></media-time-range>
	{/snippet}
</ButtonGroupText>
