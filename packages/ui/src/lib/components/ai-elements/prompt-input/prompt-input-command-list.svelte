<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CommandList } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let { class: className, ...restProps }: CommandPrimitive.ListProps = $props();
</script>

<CommandList data-slot="prompt-input-command-list" class={cn(className)} {...restProps} />
