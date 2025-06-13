<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';
	import InputArea from './input-area.svelte';
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import { Search } from '@lucide/svelte';

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

<div class="items-top flex h-[100px] border-none px-3" data-command-input-wrapper="">
	<Search class="self-center opacity-30" size="40" style="align-self: center; " />
	<div class="mr-2 h-[100px] w-2 shrink-0"></div>

	<!-- <Youtube class="mr-2 mt-6 shrink-0 opacity-50" size="70" style="color: rgba(0, 0, 0, 0.8); " /> -->
	<CommandPrimitive.Input
		class={cn(
			'custom-input flex w-full rounded-md border-none bg-transparent shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50',
			className,
		)}
		bind:ref
		bind:value
		{...restProps}
	>
		{#snippet child({ props })}
			<InputArea bind:ref={editorRef} bind:value bind:query {...props} />
		{/snippet}
	</CommandPrimitive.Input>
</div>
