<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { setEnvironmentVariable } from './environment-variables-context.svelte.js';

	let {
		class: className,
		name,
		value,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		value: string;
		children?: Snippet;
	} = $props();

	setEnvironmentVariable({
		name: () => name,
		value: () => value,
	});
</script>

<div
	data-slot="environment-variable"
	class={cn('flex items-center justify-between gap-4 px-4 py-3', className)}
	{...restProps}
>
	{@render children?.()}
</div>
