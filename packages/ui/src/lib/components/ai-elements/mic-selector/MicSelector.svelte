<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import * as Popover from '$lib/components/popover/index.js';
	import {
		MicSelectorContext,
		setMicSelectorContext,
		useAudioDevices,
	} from './mic-selector-context.svelte.js';

	interface Props {
		defaultValue?: string;
		value?: string;
		onValueChange?: (value: string | undefined) => void;
		open?: boolean;
		defaultOpen?: boolean;
		onOpenChange?: (open: boolean) => void;
		children?: Snippet;
	}

	let {
		defaultValue,
		value: controlledValue,
		onValueChange,
		open: controlledOpen,
		defaultOpen = false,
		onOpenChange,
		children,
	}: Props = $props();

	let internalValue = $state<string | undefined>(untrack(() => defaultValue));
	let internalOpen = $state(untrack(() => defaultOpen));

	const audioDevices = useAudioDevices();
	onDestroy(() => audioDevices.destroy());

	const context = new MicSelectorContext({
		devices: () => audioDevices.devices,
		value: () => controlledValue ?? internalValue,
		setValue: (val) => {
			internalValue = val;
			onValueChange?.(val);
		},
		open: () => controlledOpen ?? internalOpen,
		setOpen: (val) => {
			internalOpen = val;
			onOpenChange?.(val);
		},
	});
	setMicSelectorContext(context);

	$effect(() => {
		if (context.open && !audioDevices.hasPermission && !audioDevices.loading) {
			audioDevices.loadDevices();
		}
	});
</script>

<Popover.Root bind:open={context.open}>
	{@render children?.()}
</Popover.Root>
