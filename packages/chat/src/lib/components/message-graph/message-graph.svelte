<script lang="ts">
	import FitViewOnChange from '$lib/components/message-graph/fit-view-on-change.svelte';
	import LoadMoreNode from '$lib/components/message-graph/load-more-node.svelte';
	import MessageNode from '$lib/components/message-graph/message-node.svelte';
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
	import type { MessageTreeNode } from '$lib/models/tree.js';

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
		message: MessageNode,
		start: StartNode,
		skeleton: SkeletonNode,
		loadMore: LoadMoreNode,
	};
	const edgeTypes = { animated: EdgeAnimated, temporary: EdgeTemporary };

	type Node = {
		id: string;
		type: string;
		position: { x: number; y: number };
		data: MessageNodeData | StartNodeData | SkeletonNodeData | LoadMoreNodeData;
	};
	type Edge = { id: string; source: string; target: string; type: string };

	function addSkeletonPath(nodes: Node[], edges: Edge[], sourceId: string, depth: number) {
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

	function buildGraph(treeNodes: MessageTreeNode[]) {
		const nodes: Node[] = [];
		const edges: Edge[] = [];

		const startId = '__start__';
		nodes.push({
			id: startId,
			type: 'start',
			position: { x: 0, y: 0 },
			data: { label: startLabel, handles: { target: false, source: true } },
		});

		if (treeNodes.length === 0) {
			if (thread?.treeLoading) {
				addSkeletonPath(nodes, edges, startId, 2);
			}
			return { nodes, edges };
		}

		const maxLevel = Math.max(...treeNodes.map((n) => n.depth));
		const hasMoreLevels = thread?.treeHasMore ?? false;
		const loadingMore = thread?.treeLoading ?? false;

		const childrenMap = new Map<string, MessageTreeNode[]>();
		for (const node of treeNodes) {
			const parentKey = node.parentId ?? '__root__';
			const list = childrenMap.get(parentKey);
			if (list) {
				list.push(node);
			} else {
				childrenMap.set(parentKey, [node]);
			}
		}

		const nodeYPositions = new Map<string, number>();

		function layoutChildren(parentId: string | null, parentY: number): void {
			const key = parentId ?? '__root__';
			const children = childrenMap.get(key);
			if (!children) return;

			const totalHeight = (children.length - 1) * NODE_Y_GAP;
			const startY = parentY - totalHeight / 2;

			for (let i = 0; i < children.length; i++) {
				const child = children[i];
				const y = startY + i * NODE_Y_GAP;
				nodeYPositions.set(child.id, y);
				layoutChildren(child.id, y);
			}
		}

		layoutChildren(null, 0);

		const leafNodesAtBoundary: MessageTreeNode[] = [];

		for (const node of treeNodes) {
			const hasChildren = childrenMap.has(node.id);
			const hasSiblings = node.siblingCount > 1;
			const isAtBoundary = hasMoreLevels && node.depth === maxLevel && !hasChildren;

			nodes.push({
				id: node.id,
				type: 'message',
				position: {
					x: (node.depth + 1) * NODE_X_GAP,
					y: nodeYPositions.get(node.id) ?? 0,
				},
				data: {
					role: node.messageType === 'human' ? 'user' : 'assistant',
					content: node.content,
					siblingLabel: hasSiblings
						? `${node.siblingIndex + 1} / ${node.siblingCount}`
						: undefined,
					handles: { target: true, source: hasChildren || isAtBoundary },
					ondblclick: onMessageDblClick ? () => onMessageDblClick(node.id) : undefined,
				},
			});

			if (isAtBoundary) {
				leafNodesAtBoundary.push(node);
			}

			const sourceId = node.parentId ?? startId;
			const active = activeMessageIds.has(node.id);
			edges.push({
				id: `e-${sourceId}-${node.id}`,
				source: sourceId,
				target: node.id,
				type: active ? 'animated' : 'temporary',
			});
		}

		if (hasMoreLevels && leafNodesAtBoundary.length > 0) {
			const avgY =
				leafNodesAtBoundary.reduce((sum, n) => sum + (nodeYPositions.get(n.id) ?? 0), 0) /
				leafNodesAtBoundary.length;

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

			for (const leaf of leafNodesAtBoundary) {
				edges.push({
					id: `e-${leaf.id}-${loadMoreId}`,
					source: leaf.id,
					target: loadMoreId,
					type: 'temporary',
				});
			}
		}

		return { nodes, edges };
	}

	const graphData = $derived(buildGraph(thread?.treeNodes ?? []));
</script>

<div class="h-full w-full {className ?? ''}">
	<Canvas nodes={graphData.nodes} edges={graphData.edges} {nodeTypes} {edgeTypes} minZoom={0.01}>
		<FitViewOnChange nodeCount={graphData.nodes.length} />
	</Canvas>
</div>
