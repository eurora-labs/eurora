<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandGroup } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let {
		class: className,
		...restProps
	}: CommandPrimitive.GroupProps & {
		heading?: string;
	} = $props();
</script>

<CommandGroup data-slot="prompt-input-command-group" class={cn(className)} {...restProps} />
