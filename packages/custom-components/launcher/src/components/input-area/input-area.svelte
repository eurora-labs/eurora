<script lang="ts" module>
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core';
	import type { ClassValue } from 'svelte/elements';

	export interface Props {
		ref?: ProsemirrorEditor;
		query?: Query;
		value?: string;
		class?: ClassValue;
	}
</script>

<script lang="ts">
	import { cn } from '@eurora/ui/utils';
	import { onMount } from 'svelte';

	let {
		ref = $bindable(),
		query = $bindable(undefined),
		value = $bindable(''),
		class: className,
		...restProps
	}: Props = $props();

	onMount(() => {
		if (!query) {
			query = {
				text: '',
				extensions: []
			};
		}
		ref?.sendQuery(query);
	});
</script>

<ProsemirrorEditor
	bind:this={ref}
	bind:value
	class={cn(
		className,
		'ProsemirrorEditor h-fit min-h-[100px] border-none pt-[15px] text-[40px] leading-[100px] text-black shadow-none'
	)}
	{...restProps}
/>

<!-- <textarea
	bind:value
	class={cn(
		'border-input focus-visible:ring-ring flex w-full resize-none overflow-hidden rounded-md border bg-transparent px-3 py-2 shadow-sm focus-visible:outline-none focus-visible:ring-1 disabled:cursor-not-allowed disabled:opacity-50',
		className
	)}
	{...restProps}
></textarea> -->

<!-- <textarea
	bind:this={ref}
	bind:value
	class={cn(
		'border-input focus-visible:ring-ring flex w-full resize-none overflow-hidden rounded-md border bg-transparent px-3 py-2 shadow-sm focus-visible:outline-none focus-visible:ring-1 disabled:cursor-not-allowed disabled:opacity-50',
		className
	)}
	oninput={adjustHeight}
	{...restProps}
></textarea> -->
<style>
	textarea::placeholder {
		color: rgba(0, 0, 0, 0.25);
		text-align: start;
	}
	.ProsemirrorEditor {
		color: rgba(0, 0, 0, 0.8);
	}
</style>
