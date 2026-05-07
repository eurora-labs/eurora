<script lang="ts">
	import { untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import * as Dialog from '$lib/components/dialog/index.js';
	import {
		VoiceSelectorContext,
		setVoiceSelectorContext,
	} from './voice-selector-context.svelte.js';

	interface Props {
		value?: string;
		defaultValue?: string;
		onValueChange?: (value: string | undefined) => void;
		open?: boolean;
		defaultOpen?: boolean;
		onOpenChange?: (open: boolean) => void;
		children?: Snippet;
	}

	let {
		value: valueProp,
		defaultValue,
		onValueChange,
		open: openProp,
		defaultOpen = false,
		onOpenChange,
		children,
	}: Props = $props();

	let internalValue = $state<string | undefined>(untrack(() => defaultValue));
	let internalOpen = $state(untrack(() => defaultOpen));

	const context = new VoiceSelectorContext({
		value: () => valueProp ?? internalValue,
		setValue: (val) => {
			internalValue = val;
			onValueChange?.(val);
		},
		open: () => openProp ?? internalOpen,
		setOpen: (val) => {
			internalOpen = val;
			onOpenChange?.(val);
		},
	});
	setVoiceSelectorContext(context);
</script>

<Dialog.Root bind:open={context.open}>
	{@render children?.()}
</Dialog.Root>
