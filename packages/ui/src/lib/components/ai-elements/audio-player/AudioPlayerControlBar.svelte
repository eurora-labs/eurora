<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { ButtonGroup } from '$lib/components/button-group/index.js';

	interface Props extends HTMLAttributes<HTMLElement> {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<media-control-bar data-slot="audio-player-control-bar" {...restProps}>
	<ButtonGroup orientation="horizontal">
		{@render children?.()}
	</ButtonGroup>
</media-control-bar>
