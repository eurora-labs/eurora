export interface MessageTreeNode {
	id: string;
	parentId: string | null;
	messageType: string;
	content: string;
	depth: number;
	siblingCount: number;
	siblingIndex: number;
}

export interface MessageTreeResponse {
	nodes: MessageTreeNode[];
	hasMore: boolean;
}
