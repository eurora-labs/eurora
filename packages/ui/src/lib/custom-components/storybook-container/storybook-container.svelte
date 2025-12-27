<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	export interface Props {
		children: Snippet;
		class?: string;
	}

	let { children, class: className }: Props = $props();
</script>

<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg bg-gray-300">
	<div
		class="absolute inset-0 bg-cover bg-center bg-no-repeat blur-2xl"
		style="background-image: url('sample_background.jpg'); background-color: #f5f5f5;"
		role="presentation"
		aria-hidden="true"
	></div>
	<div class={cn('relative z-10 flex min-h-full items-center justify-center p-6', className)}>
		{@render children()}
	</div>
</div>

<style lang="postcss">
	@reference "tailwindcss";
</style>
