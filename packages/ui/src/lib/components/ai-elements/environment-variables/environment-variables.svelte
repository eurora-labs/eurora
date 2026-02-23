<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { setEnvironmentVariables } from './environment-variables-context.svelte.js';

	let {
		class: className,
		showValues: controlledShowValues,
		defaultShowValues = false,
		onShowValuesChange,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		showValues?: boolean;
		defaultShowValues?: boolean;
		onShowValuesChange?: (show: boolean) => void;
		children?: Snippet;
	} = $props();

	let internalShowValues = $state(defaultShowValues);
	let resolvedShowValues = $derived(controlledShowValues ?? internalShowValues);

	setEnvironmentVariables({
		showValues: () => resolvedShowValues,
		setShowValues: (show: boolean) => {
			internalShowValues = show;
			onShowValuesChange?.(show);
		},
	});
</script>

<div
	data-slot="environment-variables"
	class={cn('rounded-lg border bg-background', className)}
	{...restProps}
>
	{@render children?.()}
</div>
