<script lang="ts">
	import FitViewOnChange from '$lib/components/message-graph/fit-view-on-change.svelte';
	import LoadMoreNode from '$lib/components/message-graph/load-more-node.svelte';
	import MessageGraphNode from '$lib/components/message-graph/message-node.svelte';
	import SkeletonNode from '$lib/components/message-graph/skeleton-node.svelte';
	import StartNode from '$lib/components/message-graph/start-node.svelte';
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Canvas } from '@eurora/ui/components/ai-elements/canvas/index';
	import { EdgeAnimated, EdgeTemporary } from '@eurora/ui/components/ai-elements/edge/index';
	import type { LoadMoreNodeData } from '$lib/components/message-graph/load-more-node.svelte';
	import type { MessageNodeData } from '$lib/components/message-graph/message-node.svelte';
	import type { SkeletonNodeData } from '$lib/components/message-graph/skeleton-node.svelte';
	import type { StartNodeData } from '$lib/components/message-graph/start-node.svelte';
	import type { MessageNode } from '$lib/models/messages/index.js';

	interface Props {
		startLabel?: string;
		onMessageDblClick?: (messageId: string) => void;
		class?: string;
	}

	let { startLabel = 'Start', onMessageDblClick, class: className }: Props = $props();

	const chatService = inject(CHAT_SERVICE);

	const threadId = $derived(chatService.activeThreadId);
	const thread = $derived(chatService.activeThread);
	const activeMessageIds = $derived(new Set(thread?.messages.map((n) => n.message.id) ?? []));

	$effect(() => {
		if (threadId && !thread?.treeLoaded && !thread?.treeLoading) {
			chatService.loadTree(threadId);
		}
	});

	const NODE_X_GAP = 450;
	const NODE_Y_GAP = 250;

	const nodeTypes = {
		message: MessageGraphNode,
		start: StartNode,
		skeleton: SkeletonNode,
		loadMore: LoadMoreNode,
	};
	const edgeTypes = { animated: EdgeAnimated, temporary: EdgeTemporary };

	type FlowNode = {
		id: string;
		type: string;
		position: { x: number; y: number };
		data: MessageNodeData | StartNodeData | SkeletonNodeData | LoadMoreNodeData;
	};
	type FlowEdge = { id: string; source: string; target: string; type: string };

	function addSkeletonPath(
		nodes: FlowNode[],
		edges: FlowEdge[],
		sourceId: string,
		depth: number,
	) {
		let prevId = sourceId;
		for (let i = 0; i < depth; i++) {
			const role: 'user' | 'assistant' = i % 2 === 0 ? 'user' : 'assistant';
			const id = `__skeleton_${i}__`;
			const isLast = i === depth - 1;
			nodes.push({
				id,
				type: 'skeleton',
				position: { x: (i + 1) * NODE_X_GAP, y: 0 },
				data: { role, handles: { target: true, source: !isLast } },
			});
			edges.push({
				id: `e-${prevId}-${id}`,
				source: prevId,
				target: id,
				type: 'animated',
			});
			prevId = id;
		}
	}

	function getTextContent(node: MessageNode): string {
		if (!('content' in node.message)) return '';
		for (const block of node.message.content) {
			if (block.type === 'text') return block.text;
		}
		return '';
	}

	function buildGraph(treeRoots: MessageNode[]) {
		const nodes: FlowNode[] = [];
		const edges: FlowEdge[] = [];

		const startId = '__start__';
		nodes.push({
			id: startId,
			type: 'start',
			position: { x: 0, y: 0 },
			data: { label: startLabel, handles: { target: false, source: true } },
		});

		if (treeRoots.length === 0) {
			if (thread?.treeLoading) {
				addSkeletonPath(nodes, edges, startId, 2);
			}
			return { nodes, edges };
		}

		const maxLevel = thread?.treeLoadedEndLevel ?? 0;
		const hasMoreLevels = thread?.treeHasMore ?? false;
		const loadingMore = thread?.treeLoading ?? false;

		const yPositions = new Map<string, number>();
		const boundaryLeaves: MessageNode[] = [];

		function layoutSubtree(siblings: MessageNode[], parentY: number): void {
			const totalHeight = (siblings.length - 1) * NODE_Y_GAP;
			const startY = parentY - totalHeight / 2;

			for (let i = 0; i < siblings.length; i++) {
				const node = siblings[i];
				const y = startY + i * NODE_Y_GAP;
				yPositions.set(node.message.id, y);
				layoutSubtree(node.children, y);
			}
		}

		layoutSubtree(treeRoots, 0);

		function addNodes(siblings: MessageNode[], parentId: string | null): void {
			for (const node of siblings) {
				const id = node.message.id;
				const hasChildren = node.children.length > 0;
				const hasSiblings = siblings.length > 1;
				const isAtBoundary = hasMoreLevels && node.depth === maxLevel && !hasChildren;

				nodes.push({
					id,
					type: 'message',
					position: {
						x: (node.depth + 1) * NODE_X_GAP,
						y: yPositions.get(id) ?? 0,
					},
					data: {
						role: node.message.type === 'human' ? 'user' : 'assistant',
						content: getTextContent(node),
						siblingLabel: hasSiblings
							? `${node.siblingIndex + 1} / ${siblings.length}`
							: undefined,
						handles: { target: true, source: hasChildren || isAtBoundary },
						ondblclick: onMessageDblClick ? () => onMessageDblClick(id) : undefined,
					},
				});

				if (isAtBoundary) boundaryLeaves.push(node);

				const sourceId = parentId ?? startId;
				const active = activeMessageIds.has(id);
				edges.push({
					id: `e-${sourceId}-${id}`,
					source: sourceId,
					target: id,
					type: active ? 'animated' : 'temporary',
				});

				addNodes(node.children, id);
			}
		}

		addNodes(treeRoots, null);

		if (hasMoreLevels && boundaryLeaves.length > 0) {
			const avgY =
				boundaryLeaves.reduce((sum, n) => sum + (yPositions.get(n.message.id) ?? 0), 0) /
				boundaryLeaves.length;

			const loadMoreId = '__load_more__';
			nodes.push({
				id: loadMoreId,
				type: 'loadMore',
				position: { x: (maxLevel + 2) * NODE_X_GAP, y: avgY },
				data: {
					loading: loadingMore,
					handles: { target: true, source: false },
					onclick: () => {
						if (threadId) chatService.loadMoreTreeLevels(threadId);
					},
				},
			});

			for (const leaf of boundaryLeaves) {
				edges.push({
					id: `e-${leaf.message.id}-${loadMoreId}`,
					source: leaf.message.id,
					target: loadMoreId,
					type: 'temporary',
				});
			}
		}

		return { nodes, edges };
	}

	const graphData = $derived(buildGraph(thread?.treeRoots ?? []));
</script>

<div class="h-full w-full {className ?? ''}">
	<Canvas nodes={graphData.nodes} edges={graphData.edges} {nodeTypes} {edgeTypes} minZoom={0.01}>
		<FitViewOnChange nodeCount={graphData.nodes.length} />
	</Canvas>
</div>
