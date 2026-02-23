<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Command } from '$lib/components/command/index.js';
	import { Command as CommandPrimitive } from 'bits-ui';

	let {
		class: className,
		...restProps
	}: CommandPrimitive.RootProps = $props();
</script>

<Command data-slot="prompt-input-command" class={cn(className)} {...restProps} />
