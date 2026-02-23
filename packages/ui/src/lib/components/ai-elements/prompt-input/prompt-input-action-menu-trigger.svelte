<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { DropdownMenuTrigger } from '$lib/components/dropdown-menu/index.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import PlusIcon from '@lucide/svelte/icons/plus';

	let {
		class: className,
		variant = 'ghost',
		size = 'icon-sm',
		children,
		...restProps
	}: ButtonProps & {
		children?: Snippet;
	} = $props();
</script>

<DropdownMenuTrigger>
	{#snippet child({ props: triggerProps })}
		<Button
			data-slot="prompt-input-action-menu-trigger"
			class={cn(className)}
			{size}
			type="button"
			{variant}
			{...triggerProps}
			{...restProps}
		>
			{#if children}
				{@render children()}
			{:else}
				<PlusIcon class="size-4" />
			{/if}
		</Button>
	{/snippet}
</DropdownMenuTrigger>
