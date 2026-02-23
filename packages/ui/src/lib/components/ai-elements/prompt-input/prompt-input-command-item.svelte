<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandItem } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let { class: className, ...restProps }: CommandPrimitive.ItemProps = $props();
</script>

<CommandItem data-slot="prompt-input-command-item" class={cn(className)} {...restProps} />
