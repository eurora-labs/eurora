<script lang="ts">
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

	let context = new VoiceSelectorContext({
		value: valueProp ?? defaultValue,
		open: openProp ?? defaultOpen,
		onValueChange,
		onOpenChange,
	});

	$effect(() => {
		if (valueProp !== undefined) {
			context.value = valueProp;
		}
	});

	$effect(() => {
		if (openProp !== undefined) {
			context.open = openProp;
		}
	});

	setVoiceSelectorContext(context);
</script>

<Dialog.Root bind:open={context.open}>
	{@render children?.()}
</Dialog.Root>
