import Editor from '$lib/Editor.svelte';

export interface QueryAssets {
	text: string;
	assets: string[];
}

/**
 * Processes the editor query to extract text content and asset identifiers
 * @param editorRef - The ProseMirror editor instance
 * @returns QueryAssets object containing text and assets array
 * @throws Error if editor view is not available
 */
export function processQuery(editorRef: Editor): QueryAssets {
	const query: QueryAssets = { text: '', assets: [] };
	const view = editorRef.view;
	if (!view) {
		throw new Error('No view found');
	}

	// Get the editor state as JSON to match the expected structure
	const stateJson = view.state.toJSON();

	// Recursive function to process nodes based on the JSON structure
	function processNodeJson(node: any) {
		// If it's a text node, add its text content to the query
		if (node.type === 'text' && node.text) {
			query.text += ' ' + node.text + ' ';
		} // If it's any other node with a type that looks like a UUID (not doc or paragraph)
		// add it to the query as an identifier
		// Note: The 'type' property is used as the node's ID as shown in the example JSON
		else if (node.type && node.type !== 'doc' && node.type !== 'paragraph') {
			// If the type looks like a UUID (has hyphens and is long), add it to the query
			if (node.type.includes('-') || node.type.length > 10) {
				query.assets.push(node.attrs?.id ?? '');
				query.text += ' ' + node.attrs?.name + ' ';
			}
		}

		// Process child nodes if they exist
		if (node.content && Array.isArray(node.content)) {
			node.content.forEach(processNodeJson);
		}
	}

	// Process the document content
	if (stateJson.doc?.content) {
		stateJson.doc.content.forEach(processNodeJson);
	}

	return query;
}

/**
 * Clears the editor query
 * @param editorRef - The ProseMirror editor instance
 * @throws Error if editor view is not available
 */
export async function clearQuery(editorRef: Editor) {
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		tr.delete(0, state.doc.content.size);
		dispatch?.(tr);
	});
}

/**
 * Checks if a node type name looks like a UUID (registered extension node)
 * @param typeName - The node type name to check
 * @returns true if it looks like a UUID extension node
 */
function isExtensionNodeType(typeName: string): boolean {
	// Extension nodes use UUIDs as their type names
	// They contain hyphens and are longer than 10 characters
	return (
		typeName !== 'doc' &&
		typeName !== 'paragraph' &&
		typeName !== 'text' &&
		(typeName.includes('-') || typeName.length > 10)
	);
}

/**
 * Clears only the extension nodes (assets) from the editor while preserving text content
 * @param editorRef - The ProseMirror editor instance
 * @throws Error if editor view is not available
 */
export async function clearExtensionNodes(editorRef: Editor) {
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const nodesToDelete: { from: number; to: number }[] = [];

		// Walk through the document and find all extension nodes
		state.doc.descendants((node, pos) => {
			if (isExtensionNodeType(node.type.name)) {
				nodesToDelete.push({
					from: pos,
					to: pos + node.nodeSize,
				});
			}
			return true; // Continue traversing
		});

		// Delete nodes in reverse order to maintain position accuracy
		nodesToDelete.reverse().forEach(({ from, to }) => {
			tr.delete(from, to);
		});

		if (nodesToDelete.length > 0) {
			dispatch?.(tr);
		}
	});
}
