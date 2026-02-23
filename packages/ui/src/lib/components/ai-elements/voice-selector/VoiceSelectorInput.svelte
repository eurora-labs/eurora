<script lang="ts">
	import { cn } from '$lib/utils.js';
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		placeholder?: string;
	}

	let { class: className, placeholder }: Props = $props();
</script>

<Command.Input
	data-slot="voice-selector-input"
	class={cn('h-auto py-3.5', className)}
	{placeholder}
/>
