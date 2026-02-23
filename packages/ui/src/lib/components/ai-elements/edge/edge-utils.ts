import { Position } from '@xyflow/svelte';
import type { InternalNode } from '@xyflow/svelte';

export const getHandleCoordsByPosition = (
	node: InternalNode,
	handlePosition: Position,
): readonly [number, number] => {
	const handleType = handlePosition === Position.Left ? 'target' : 'source';

	const handle = node.internals.handleBounds?.[handleType]?.find(
		(h) => h.position === handlePosition,
	);

	if (!handle) {
		return [0, 0] as const;
	}

	let offsetX = handle.width / 2;
	let offsetY = handle.height / 2;

	switch (handlePosition) {
		case Position.Left: {
			offsetX = 0;
			break;
		}
		case Position.Right: {
			offsetX = handle.width;
			break;
		}
		case Position.Top: {
			offsetY = 0;
			break;
		}
		case Position.Bottom: {
			offsetY = handle.height;
			break;
		}
		default: {
			throw new Error(`Invalid handle position: ${handlePosition}`);
		}
	}

	const x = node.internals.positionAbsolute.x + handle.x + offsetX;
	const y = node.internals.positionAbsolute.y + handle.y + offsetY;

	return [x, y] as const;
};

export const getEdgeParams = (source: InternalNode, target: InternalNode) => {
	const sourcePos = Position.Right;
	const [sx, sy] = getHandleCoordsByPosition(source, sourcePos);
	const targetPos = Position.Left;
	const [tx, ty] = getHandleCoordsByPosition(target, targetPos);

	return {
		sourcePos,
		sx,
		sy,
		targetPos,
		tx,
		ty,
	};
};
