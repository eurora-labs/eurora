<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandEmpty } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let {
		class: className,
		...restProps
	}: CommandPrimitive.EmptyProps = $props();
</script>

<CommandEmpty data-slot="prompt-input-command-empty" class={cn(className)} {...restProps} />
