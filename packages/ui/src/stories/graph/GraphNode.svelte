<script lang="ts">
	import * as FlowNode from '$lib/components/ai-elements/flow-node/index';

	let {
		data,
	}: {
		data: { title: string; description: string; handles: { target: boolean; source: boolean } };
	} = $props();
</script>

<FlowNode.Root handles={data.handles}>
	<FlowNode.Header>
		<FlowNode.Title>{data.title}</FlowNode.Title>
		<FlowNode.Description>{data.description}</FlowNode.Description>
	</FlowNode.Header>
</FlowNode.Root>
