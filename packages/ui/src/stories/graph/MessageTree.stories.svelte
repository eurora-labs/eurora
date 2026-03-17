<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { Canvas } from '$lib/components/ai-elements/canvas/index';

	const { Story } = defineMeta({
		title: 'AI Elements / Graph / Message Tree',
		component: Canvas,
		parameters: {
			layout: 'fullscreen',
			controls: { disable: true },
			docs: {
				description: {
					component:
						'Displays a conversation as a graph with forked message branches and assets above nodes.',
				},
			},
		},
	});
</script>

<script lang="ts">
	import { EdgeAnimated, EdgeTemporary } from '$lib/components/ai-elements/edge/index';
	import MessageNode from './MessageNode.svelte';

	const nodeTypes = { message: MessageNode };
	const edgeTypes = { animated: EdgeAnimated, temporary: EdgeTemporary };

	const NODE_X_GAP = 450;
	const NODE_Y_GAP = 250;

	let nodes = $state([
		{
			id: 'u1',
			type: 'message',
			position: { x: 0, y: NODE_Y_GAP },
			data: {
				role: 'user',
				content: 'Can you explain how to use React hooks effectively?',
				handles: { target: false, source: true },
			},
		},
		{
			id: 'a1',
			type: 'message',
			position: { x: NODE_X_GAP, y: NODE_Y_GAP },
			data: {
				role: 'assistant',
				content:
					'React hooks let you use state and other features without classes. Key hooks include useState, useEffect, useContext, and useReducer.',
				handles: { target: true, source: true },
			},
		},

		{
			id: 'u2a',
			type: 'message',
			position: { x: NODE_X_GAP * 2, y: 0 },
			data: {
				role: 'user',
				content:
					'Yes, could you explain useCallback and useMemo in more detail? When should I use one over the other?',
				siblingLabel: '1 / 3',
				assets: [{ id: 'file-1', name: 'hooks-cheatsheet.pdf' }],
				handles: { target: true, source: true },
			},
		},
		{
			id: 'u2b',
			type: 'message',
			position: { x: NODE_X_GAP * 2, y: NODE_Y_GAP },
			data: {
				role: 'user',
				content:
					"I'm particularly interested in understanding the performance implications of useCallback and useMemo.",
				siblingLabel: '2 / 3',
				handles: { target: true, source: true },
			},
		},
		{
			id: 'u2c',
			type: 'message',
			position: { x: NODE_X_GAP * 2, y: NODE_Y_GAP * 2 },
			data: {
				role: 'user',
				content:
					'Could you dive deeper into the specific use cases where useCallback and useMemo make the biggest difference?',
				siblingLabel: '3 / 3',
				handles: { target: true, source: true },
			},
		},

		{
			id: 'a2a',
			type: 'message',
			position: { x: NODE_X_GAP * 3, y: 0 },
			data: {
				role: 'assistant',
				content:
					'useCallback memoizes functions to prevent re-renders. useMemo memoizes values to avoid expensive recalculations. Use useCallback for event handlers passed to children.',
				handles: { target: true, source: true },
			},
		},
		{
			id: 'a2b',
			type: 'message',
			position: { x: NODE_X_GAP * 3, y: NODE_Y_GAP },
			data: {
				role: 'assistant',
				content:
					'The performance impact depends on your component tree. Overusing these hooks adds overhead. Profile first, then optimize where it matters.',
				handles: { target: true, source: true },
			},
		},
		{
			id: 'a2c',
			type: 'message',
			position: { x: NODE_X_GAP * 3, y: NODE_Y_GAP * 2 },
			data: {
				role: 'assistant',
				content:
					'The biggest wins come when passing callbacks to React.memo components, or when computing derived data from large lists.',
				handles: { target: true, source: false },
			},
		},

		{
			id: 'u3a',
			type: 'message',
			position: { x: NODE_X_GAP * 4, y: -NODE_Y_GAP / 2 },
			data: {
				role: 'user',
				content: 'Can you show me a real-world example with a todo list?',
				siblingLabel: '1 / 2',
				assets: [
					{ id: 'file-2', name: 'TodoApp.tsx' },
					{ id: 'file-3', name: 'useTodos.ts' },
				],
				handles: { target: true, source: false },
			},
		},
		{
			id: 'u3b',
			type: 'message',
			position: { x: NODE_X_GAP * 4, y: NODE_Y_GAP / 2 },
			data: {
				role: 'user',
				content: 'How does this compare to using Svelte runes instead?',
				siblingLabel: '2 / 2',
				handles: { target: true, source: true },
			},
		},

		{
			id: 'a3b',
			type: 'message',
			position: { x: NODE_X_GAP * 5, y: NODE_Y_GAP / 2 },
			data: {
				role: 'assistant',
				content:
					'Svelte runes like $state and $derived handle reactivity at compile time, so you never need useCallback or useMemo equivalents.',
				handles: { target: true, source: false },
			},
		},
	]);

	let edges = $state([
		{ id: 'e-u1-a1', source: 'u1', target: 'a1', type: 'animated' },

		{ id: 'e-a1-u2a', source: 'a1', target: 'u2a', type: 'animated' },
		{ id: 'e-a1-u2b', source: 'a1', target: 'u2b', type: 'temporary' },
		{ id: 'e-a1-u2c', source: 'a1', target: 'u2c', type: 'temporary' },

		{ id: 'e-u2a-a2a', source: 'u2a', target: 'a2a', type: 'animated' },
		{ id: 'e-u2b-a2b', source: 'u2b', target: 'a2b', type: 'animated' },
		{ id: 'e-u2c-a2c', source: 'u2c', target: 'a2c', type: 'animated' },

		{ id: 'e-a2a-u3a', source: 'a2a', target: 'u3a', type: 'animated' },
		{ id: 'e-a2a-u3b', source: 'a2a', target: 'u3b', type: 'temporary' },

		{ id: 'e-u3b-a3b', source: 'u3b', target: 'a3b', type: 'animated' },
	]);
</script>

<Story name="Message Tree">
	<div class="h-[700px] w-[1200px]">
		<Canvas {nodes} {edges} {nodeTypes} {edgeTypes} />
	</div>
</Story>
