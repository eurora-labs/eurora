<script lang="ts">
	import { Canvas } from '$lib/components/ai-elements/canvas/index';
	import { EdgeAnimated, EdgeTemporary } from '$lib/components/ai-elements/edge/index';
	import FitViewOnChange from '$lib/custom-components/message-graph/fit-view-on-change.svelte';
	import MessageNode from '$lib/custom-components/message-graph/message-node.svelte';
	import SkeletonNode from '$lib/custom-components/message-graph/skeleton-node.svelte';
	import StartNode from '$lib/custom-components/message-graph/start-node.svelte';
	import type { MessageNodeData } from '$lib/custom-components/message-graph/message-node.svelte';
	import type { SkeletonNodeData } from '$lib/custom-components/message-graph/skeleton-node.svelte';
	import type { StartNodeData } from '$lib/custom-components/message-graph/start-node.svelte';

	export interface TreeNodeData {
		id: string;
		parent_message_id: string | null;
		message_type: string;
		content: string;
		level: number;
		sibling_count: number;
		sibling_index: number;
		assets?: { id: string; name: string }[] | null;
	}

	interface Props {
		treeNodes: TreeNodeData[];
		activeMessageIds?: Set<string>;
		startLabel?: string;
		loading?: boolean;
		hasMoreLevels?: boolean;
		loadingMoreLevels?: boolean;
		onmessagedblclick?: (messageId: string) => void;
		onloadmorelevels?: () => void;
		class?: string;
	}

	let {
		treeNodes,
		activeMessageIds,
		startLabel = 'Start',
		loading = false,
		hasMoreLevels = false,
		loadingMoreLevels = false,
		onmessagedblclick,
		onloadmorelevels,
		class: className,
	}: Props = $props();

	const NODE_X_GAP = 450;
	const NODE_Y_GAP = 250;

	const nodeTypes = { message: MessageNode, start: StartNode, skeleton: SkeletonNode };
	const edgeTypes = { animated: EdgeAnimated, temporary: EdgeTemporary };

	type Node = {
		id: string;
		type: string;
		position: { x: number; y: number };
		data: MessageNodeData | StartNodeData | SkeletonNodeData;
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

	const graphData = $derived.by(() => {
		const nodes: Node[] = [];
		const edges: Edge[] = [];

		const startId = '__start__';
		nodes.push({
			id: startId,
			type: 'start',
			position: { x: 0, y: 0 },
			data: {
				label: startLabel,
				handles: { target: false, source: true },
			},
		});

		if (treeNodes.length === 0) {
			if (loading) {
				addSkeletonPath(nodes, edges, startId, 2);
			}
			return { nodes, edges };
		}

		const maxLevel = Math.max(...treeNodes.map((n) => n.level));

		const childrenMap = new Map<string, TreeNodeData[]>();
		for (const node of treeNodes) {
			const parentKey = node.parent_message_id ?? '__root__';
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

		const isActive = activeMessageIds ?? new Set<string>();
		const leafNodesAtBoundary: TreeNodeData[] = [];

		for (const node of treeNodes) {
			const content =
				typeof node.content === 'string' ? node.content : JSON.stringify(node.content);

			const assets = node.assets?.map((a) => ({ id: a.id, name: a.name }));
			const hasChildren = childrenMap.has(node.id);
			const hasSiblings = node.sibling_count > 1;
			const isAtBoundary = hasMoreLevels && node.level === maxLevel && !hasChildren;

			nodes.push({
				id: node.id,
				type: 'message',
				position: {
					x: (node.level + 1) * NODE_X_GAP,
					y: nodeYPositions.get(node.id) ?? 0,
				},
				data: {
					role: node.message_type === 'human' ? 'user' : 'assistant',
					content,
					siblingLabel: hasSiblings
						? `${node.sibling_index + 1} / ${node.sibling_count}`
						: undefined,
					assets,
					handles: {
						target: true,
						source: hasChildren || isAtBoundary,
					},
					ondblclick: onmessagedblclick ? () => onmessagedblclick(node.id) : undefined,
				},
			});

			if (isAtBoundary) {
				leafNodesAtBoundary.push(node);
			}

			const sourceId = node.parent_message_id ?? startId;
			const active = isActive.size === 0 || isActive.has(node.id);
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
				type: 'message',
				position: {
					x: (maxLevel + 2) * NODE_X_GAP,
					y: avgY,
				},
				data: {
					role: 'assistant',
					label: loadingMoreLevels ? 'Loading...' : 'Load more',
					content: '',
					handles: { target: true, source: false },
					ondblclick: onloadmorelevels,
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
	});
</script>

<div class="h-full w-full {className ?? ''}">
	<Canvas nodes={graphData.nodes} edges={graphData.edges} {nodeTypes} {edgeTypes}>
		<FitViewOnChange nodeCount={graphData.nodes.length} />
	</Canvas>
</div>
