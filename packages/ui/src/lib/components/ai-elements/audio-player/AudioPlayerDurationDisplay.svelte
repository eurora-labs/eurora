<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { ButtonGroupText } from '$lib/components/button-group/index.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<ButtonGroupText class="bg-transparent">
	{#snippet child({ props })}
		<media-duration-display
			data-slot="audio-player-duration-display"
			{...props}
			class={cn(props.class as string, 'tabular-nums', className)}
			{...restProps}
		></media-duration-display>
	{/snippet}
</ButtonGroupText>
