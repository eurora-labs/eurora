<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { InputGroupInput } from '$lib/components/input-group/index.js';
	import { useSnippet } from './snippet-context.svelte.js';

	let {
		class: className,
		...restProps
	}: HTMLAttributes<HTMLInputElement> & Record<string, unknown> = $props();

	const ctx = useSnippet();
</script>

<InputGroupInput
	data-slot="snippet-input"
	class={cn('text-foreground', className)}
	readonly
	value={ctx.code}
	{...restProps}
/>
