<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLInputAttributes {}

	let { class: className, ...restProps }: Props = $props();
</script>

<Command.Input data-slot="voice-selector-input" class={cn('h-auto py-3.5', className)} {...restProps} />
