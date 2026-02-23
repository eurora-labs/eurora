<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Avatar, AvatarFallback } from '$lib/components/avatar/index.js';

	let {
		class: className,
		initials,
		...restProps
	}: {
		class?: string;
		initials: string;
	} & Record<string, unknown> = $props();
</script>

<Avatar data-slot="commit-author-avatar" class={cn('size-8', className)} {...restProps}>
	<AvatarFallback class="text-xs">{initials}</AvatarFallback>
</Avatar>
