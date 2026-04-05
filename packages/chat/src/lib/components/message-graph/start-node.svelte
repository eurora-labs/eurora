<script lang="ts">
	import * as FlowNode from '@eurora/ui/components/ai-elements/flow-node/index';

	export interface StartNodeData {
		label: string;
		handles: { target: boolean; source: boolean };
	}

	let { data }: { data: StartNodeData } = $props();
</script>

<div class="flex flex-col items-center gap-2">
	<FlowNode.Root
		handles={data.handles}
		class="bg-secondary text-secondary-foreground flex items-center justify-center px-6 py-3 text-sm font-medium shadow-sm"
	>
		{data.label}
	</FlowNode.Root>
</div>
