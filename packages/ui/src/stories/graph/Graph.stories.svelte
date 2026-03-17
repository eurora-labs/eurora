<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { Canvas } from '$lib/components/ai-elements/canvas/index';

	const { Story } = defineMeta({
		title: 'AI Elements / Graph / Basic',
		component: Canvas,
		parameters: {
			layout: 'fullscreen',
			controls: { disable: true },
			docs: {
				description: {
					component:
						'Basic graph showing Canvas, FlowNode, and Edge components from ai-elements.',
				},
			},
		},
	});
</script>

<script lang="ts">
	import { EdgeAnimated, EdgeTemporary } from '$lib/components/ai-elements/edge/index';
	import GraphNode from './GraphNode.svelte';

	const nodeTypes = { custom: GraphNode };
	const edgeTypes = { animated: EdgeAnimated, temporary: EdgeTemporary };

	let nodes = $state([
		{
			id: '1',
			type: 'custom',
			position: { x: 0, y: 0 },
			data: {
				title: 'Input',
				description: 'User prompt entry point',
				handles: { target: false, source: true },
			},
		},
		{
			id: '2',
			type: 'custom',
			position: { x: 400, y: -80 },
			data: {
				title: 'LLM',
				description: 'Language model processing',
				handles: { target: true, source: true },
			},
		},
		{
			id: '3',
			type: 'custom',
			position: { x: 400, y: 120 },
			data: {
				title: 'Tool Call',
				description: 'External tool execution',
				handles: { target: true, source: true },
			},
		},
		{
			id: '4',
			type: 'custom',
			position: { x: 800, y: 0 },
			data: {
				title: 'Output',
				description: 'Final response',
				handles: { target: true, source: false },
			},
		},
	]);

	let edges = $state([
		{ id: 'e1-2', source: '1', target: '2', type: 'animated' },
		{ id: 'e1-3', source: '1', target: '3', type: 'temporary' },
		{ id: 'e2-4', source: '2', target: '4', type: 'animated' },
		{ id: 'e3-4', source: '3', target: '4', type: 'temporary' },
	]);
</script>

<Story name="Graph">
	<div class="h-[600px] w-[1000px]">
		<Canvas {nodes} {edges} {nodeTypes} {edgeTypes} />
	</div>
</Story>
