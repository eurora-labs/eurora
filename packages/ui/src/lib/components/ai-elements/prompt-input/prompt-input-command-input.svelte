<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandInput } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let { class: className, ...restProps }: CommandPrimitive.InputProps = $props();
</script>

<CommandInput data-slot="prompt-input-command-input" class={cn(className)} {...restProps} />
