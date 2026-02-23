<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes } from 'svelte/elements';
	import BookIcon from '@lucide/svelte/icons/book';

	let { href, title, children, ...restProps }: HTMLAnchorAttributes & { children?: Snippet } =
		$props();
</script>

<a
	data-slot="source"
	class="flex items-center gap-2"
	{href}
	rel="noreferrer"
	target="_blank"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<BookIcon class="h-4 w-4" />
		<span class="block font-medium">{title}</span>
	{/if}
</a>
