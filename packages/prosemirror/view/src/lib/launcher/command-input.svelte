<script lang="ts" module>
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import { Command as CommandPrimitive } from 'bits-ui';
	interface Props extends CommandPrimitive.InputProps {
		query?: Query;
		editorRef?: ProsemirrorEditor;
		iconSize?: number;
		height?: string;
		onheightchange?: (height: number) => void;
	}
</script>

<script lang="ts">
	import InputArea from '$lib/launcher/input-area.svelte';
	import { cn } from '$lib/utils';
	import SearchIcon from '@lucide/svelte/icons/search';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		height = $bindable('100px'),
		query = $bindable(undefined),
		onheightchange,
		editorRef = $bindable(),
		iconSize = $bindable(40),
		...restProps
	}: Props = $props();
</script>

<div
	class={cn('flex border-none px-3 w-full flex-row', className)}
	data-command-input-wrapper=""
	style="min-height: inherit"
>
	<div class="flex items-center justify-center max-h-25">
		<SearchIcon class="shrink-0 text-muted-foreground" size={iconSize} />
	</div>
	<div class="mr-2 w-2 shrink-0"></div>
	<div
		class="flex flex-col justify-center items-center w-full h-full py-0"
		style="min-height: inherit"
	>
		<CommandPrimitive.Input
			class="custom-input flex w-full rounded-md border-none bg-transparent shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50 min-h-[1em]"
			bind:ref
			bind:value
			{...restProps}
		>
			{#snippet child({ props })}
				<InputArea bind:ref={editorRef} bind:value bind:query {onheightchange} {...props} />
			{/snippet}
		</CommandPrimitive.Input>
	</div>
</div>
