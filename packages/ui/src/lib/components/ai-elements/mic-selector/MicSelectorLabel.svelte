<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { parseDeviceLabel } from './mic-selector-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		device: MediaDeviceInfo;
	}

	let { device, class: className, ...restProps }: Props = $props();

	let parsed = $derived(parseDeviceLabel(device.label));
</script>

<span data-slot="mic-selector-label" class={className} {...restProps}>
	{#if parsed.deviceId}
		<span>{parsed.name}</span>
		<span class="text-muted-foreground"> ({parsed.deviceId})</span>
	{:else}
		{device.label}
	{/if}
</span>
