<script lang="ts">
	import * as FlowNode from '@eurora/ui/components/ai-elements/flow-node/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';

	export interface LoadMoreNodeData {
		loading: boolean;
		handles: { target: boolean; source: boolean };
		onclick?: () => void;
	}

	let { data }: { data: LoadMoreNodeData } = $props();
</script>

<div class="flex flex-col items-center gap-2">
	<FlowNode.Root handles={data.handles} class="border-none bg-transparent shadow-none">
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
	</FlowNode.Root>
</div>
