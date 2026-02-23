<script lang="ts">
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		placeholder?: string;
	}

	let { class: className, placeholder = 'Search microphones...' }: Props = $props();
</script>

<Command.Input data-slot="mic-selector-input" class={className} {placeholder} />
