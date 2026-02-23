<script lang="ts">
	import { DropdownMenuItem } from '$lib/components/dropdown-menu/index.js';
	import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';
	import ImageIcon from '@lucide/svelte/icons/image';
	import { usePromptInputAttachments } from './prompt-input-context.svelte.js';

	let {
		label = 'Add photos or files',
		...restProps
	}: DropdownMenuPrimitive.ItemProps & {
		label?: string;
	} = $props();

	const attachments = usePromptInputAttachments();

	function handleSelect(e: Event) {
		e.preventDefault();
		attachments.openFileDialog();
	}
</script>

<DropdownMenuItem
	data-slot="prompt-input-action-add-attachments"
	onSelect={handleSelect}
	{...restProps}
>
	<ImageIcon class="mr-2 size-4" />
	{label}
</DropdownMenuItem>
