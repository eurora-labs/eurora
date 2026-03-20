<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';

	export interface StartNodeData {
		label: string;
		handles: { target: boolean; source: boolean };
	}

	let { data }: { data: StartNodeData } = $props();
</script>

<div class="flex flex-col items-center gap-2">
	<div
		class="bg-secondary text-secondary-foreground relative flex items-center justify-center rounded-md px-6 py-3 text-sm font-medium shadow-sm"
	>
		{#if data.handles.target}
			<Handle position={Position.Left} type="target" />
		{/if}
		{#if data.handles.source}
			<Handle position={Position.Right} type="source" />
		{/if}
		{data.label}
	</div>
</div>
