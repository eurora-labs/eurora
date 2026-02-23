<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Streamdown, type StreamdownProps } from 'svelte-streamdown';

	let {
		class: className,
		...restProps
	}: StreamdownProps & {
		class?: string;
	} = $props();
</script>

<Streamdown
	data-slot="message-response"
	class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}
	{...restProps}
/>
