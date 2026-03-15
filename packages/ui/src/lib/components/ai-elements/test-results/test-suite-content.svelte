<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children }: Props = $props();
</script>

<CollapsibleContent data-slot="test-suite-content" class={cn('border-t', className)}>
	<div class="divide-y">
		{@render children?.()}
	</div>
</CollapsibleContent>
