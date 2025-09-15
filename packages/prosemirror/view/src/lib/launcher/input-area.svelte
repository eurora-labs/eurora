<script lang="ts" module>
	import { Editor } from '@eurora/prosemirror-core/index';
	import type { Query } from '@eurora/prosemirror-core/index';
	import type { ClassValue } from 'svelte/elements';

	export interface Props {
		ref?: Editor;
		query?: Query;
		value?: string;
		class?: ClassValue;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils';
	import { onMount } from 'svelte';

	let {
		ref = $bindable(),
		query = $bindable(),
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

<Editor
	bind:this={ref}
	bind:value
	class={cn(className, 'ProsemirrorEditor h-fit min-h-[100px] shadow-none')}
	{...restProps}
/>
