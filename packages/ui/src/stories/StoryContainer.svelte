<script lang="ts">
	import type { Snippet } from 'svelte';

	export interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
	<div
		class="absolute inset-0 bg-cover bg-center bg-no-repeat"
		style="background-image: url('sample_background.jpg'), url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48ZGVmcz48cGF0dGVybiBpZD0iZ3JpZCIgd2lkdGg9IjEwIiBoZWlnaHQ9IjEwIiBwYXR0ZXJuVW5pdHM9InVzZXJTcGFjZU9uVXNlIj48cGF0aCBkPSJNIDEwIDAgTCAwIDAgMCAxMCIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjZGRkIiBzdHJva2Utd2lkdGg9IjAuNSIvPjwvcGF0dGVybj48L2RlZnM+PHJlY3Qgd2lkdGg9IjEwMCUiIGhlaWdodD0iMTAwJSIgZmlsbD0idXJsKCNncmlkKSIvPjwvc3ZnPg=='); background-color: #f5f5f5;"
		role="presentation"
		aria-hidden="true"
	></div>
	<div class="relative z-10 flex min-h-full items-center justify-center p-6">
		{@render children()}
	</div>
</div>
