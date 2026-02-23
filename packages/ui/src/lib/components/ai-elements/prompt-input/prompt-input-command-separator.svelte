<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandSeparator } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let { class: className, ...restProps }: CommandPrimitive.SeparatorProps = $props();
</script>

<CommandSeparator data-slot="prompt-input-command-separator" class={cn(className)} {...restProps} />
