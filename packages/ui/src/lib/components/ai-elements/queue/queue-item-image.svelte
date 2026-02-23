<script lang="ts">
	import type { HTMLImgAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let { class: className, alt = '', ...restProps }: HTMLImgAttributes = $props();
</script>

<img
	data-slot="queue-item-image"
	{alt}
	class={cn('h-8 w-8 rounded border object-cover', className)}
	height={32}
	width={32}
	{...restProps}
/>
