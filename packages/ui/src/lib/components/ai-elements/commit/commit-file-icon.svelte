<script lang="ts">
	import { cn } from '$lib/utils.js';
	import File from '@lucide/svelte/icons/file';

	let {
		class: className,
		...restProps
	}: {
		class?: string;
	} & Record<string, unknown> = $props();
</script>

<File
	data-slot="commit-file-icon"
	class={cn('size-3.5 shrink-0 text-muted-foreground', className)}
	{...restProps}
/>
