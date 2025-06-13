<script lang="ts" module>
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import type { ClassValue } from 'svelte/elements';

	export interface Props {
		ref?: ProsemirrorEditor;
		query?: Query;
		value?: string;
		class?: ClassValue;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils.js';
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
				extensions: [],
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
		'ProsemirrorEditor h-fit min-h-[100px] text-[40px] text-black/50 shadow-none',
	)}
	{...restProps}
/>
