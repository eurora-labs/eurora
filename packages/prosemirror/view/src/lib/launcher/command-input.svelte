<script lang="ts" module>
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import { Command as CommandPrimitive } from 'bits-ui';
	interface Props extends CommandPrimitive.InputProps {
		query?: Query;
		editorRef?: ProsemirrorEditor;
		iconSize?: number;
		height?: string;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils';
	import InputArea from './input-area.svelte';
	import SearchIcon from '@lucide/svelte/icons/search';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		height = $bindable('100px'),
		query = $bindable(undefined),
		editorRef = $bindable(),
		iconSize = $bindable(40),
		...restProps
	}: Props = $props();
</script>

<div
	class={cn('items-top flex min-h-[100px] border-none px-3 w-full flex-row', className)}
	data-command-input-wrapper=""
>
	<div class="flex items-center justify-center max-h-[100px]">
		<SearchIcon class="opacity-30 shrink-0 text-black/80" size={iconSize} />
	</div>
	<div class="mr-2 min-h-[100px] w-2 shrink-0"></div>
	<CommandPrimitive.Input
		class="custom-input flex w-full rounded-md border-none bg-transparent shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50 mt-[20px]"
		bind:ref
		bind:value
		{...restProps}
	>
		{#snippet child({ props })}
			<InputArea bind:ref={editorRef} bind:value bind:query {...props} />
		{/snippet}
	</CommandPrimitive.Input>
</div>
