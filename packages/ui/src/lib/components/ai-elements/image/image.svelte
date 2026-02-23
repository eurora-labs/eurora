<script lang="ts">
	import type { HTMLImgAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		base64,
		mediaType,
		alt = '',
		class: className,
		...restProps
	}: HTMLImgAttributes & { base64: string; mediaType: string } = $props();
</script>

<img
	data-slot="image"
	src="data:{mediaType};base64,{base64}"
	{alt}
	class={cn('h-auto max-w-full overflow-hidden rounded-md', className)}
	{...restProps}
/>
