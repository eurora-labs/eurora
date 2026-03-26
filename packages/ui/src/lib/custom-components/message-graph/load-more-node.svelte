<script lang="ts">
	import { Button } from '$lib/components/button/index';
	import { Spinner } from '$lib/components/spinner/index';
	import { Handle, Position } from '@xyflow/svelte';

	export interface LoadMoreNodeData {
		loading: boolean;
		handles: { target: boolean; source: boolean };
		onclick?: () => void;
	}

	let { data }: { data: LoadMoreNodeData } = $props();
</script>

<div class="flex flex-col items-center gap-2">
	<div class="relative">
		{#if data.handles.target}
			<Handle position={Position.Left} type="target" />
		{/if}
		{#if data.handles.source}
			<Handle position={Position.Right} type="source" />
		{/if}
		<Button
			variant="outline"
			size="lg"
			class="px-8 py-6 text-base"
			disabled={data.loading}
			onclick={data.onclick}
		>
			{#if data.loading}
				<Spinner />
			{:else}
				Load more
			{/if}
		</Button>
	</div>
</div>
