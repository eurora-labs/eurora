<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLInputAttributes {}

	let { ...restProps }: Props = $props();
</script>

<Command.Input data-slot="mic-selector-input" placeholder="Search microphones..." {...restProps} />
