<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';
	import InputArea from './input-area.svelte';
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import SearchIcon from '@lucide/svelte/icons/search';

	interface Props extends CommandPrimitive.InputProps {
		query?: Query;
		editorRef?: ProsemirrorEditor;
	}

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		height = $bindable('100px'),
		query = $bindable(undefined),
		editorRef = $bindable(),
		...restProps
	}: Props = $props();
</script>

<div
	class={cn('items-top flex h-[100px] border-none px-3 w-full flex-row', className)}
	data-command-input-wrapper=""
>
	<div class="flex">
		<SearchIcon class="opacity-30 shrink-0 text-black/80 mt-7" size="40" />
	</div>
	<div class="mr-2 h-[100px] w-2 shrink-0"></div>
	<CommandPrimitive.Input
		class="custom-input flex w-full rounded-md border-none bg-transparent shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50"
		bind:ref
		bind:value
		{...restProps}
	>
		{#snippet child({ props })}
			<InputArea bind:ref={editorRef} bind:value bind:query {...props} />
		{/snippet}
	</CommandPrimitive.Input>
</div>
