<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import CodeBlock from '../code-block/code-block.svelte';

	let {
		class: className,
		output,
		errorText,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		output: unknown;
		errorText?: string;
	} = $props();

	let visible = $derived(!!(output || errorText));

	let formatted = $derived(
		typeof output === 'object' ? JSON.stringify(output, null, 2) : String(output ?? ''),
	);
</script>

{#if visible}
	<div data-slot="tool-output" class={cn('space-y-2', className)} {...restProps}>
		<h4 class="font-medium text-muted-foreground text-xs uppercase tracking-wide">
			{errorText ? 'Error' : 'Result'}
		</h4>
		<div
			class={cn(
				'overflow-x-auto rounded-md text-xs [&_table]:w-full',
				errorText ? 'bg-destructive/10 text-destructive' : 'bg-muted/50 text-foreground',
			)}
		>
			{#if errorText}
				<div>{errorText}</div>
			{/if}
			<CodeBlock code={formatted} language="json" />
		</div>
	</div>
{/if}
