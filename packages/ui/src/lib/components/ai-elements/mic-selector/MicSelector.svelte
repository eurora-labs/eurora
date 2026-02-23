<script lang="ts">
	import { onDestroy } from 'svelte';
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

	let audioDevices = useAudioDevices();
	onDestroy(() => audioDevices.destroy());

	let context = new MicSelectorContext({
		value: controlledValue ?? defaultValue,
		open: controlledOpen ?? defaultOpen,
		onValueChange,
		onOpenChange,
	});

	$effect(() => {
		if (controlledValue !== undefined) {
			context.value = controlledValue;
		}
	});

	$effect(() => {
		if (controlledOpen !== undefined) {
			context.open = controlledOpen;
		}
	});

	$effect(() => {
		context.devices = audioDevices.devices;
	});

	$effect(() => {
		if (context.open && !audioDevices.hasPermission && !audioDevices.loading) {
			audioDevices.loadDevices();
		}
	});

	setMicSelectorContext(context);
</script>

<Popover.Root bind:open={context.open}>
	{@render children?.()}
</Popover.Root>
