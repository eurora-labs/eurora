import Editor from '$lib/Editor.svelte';

export interface QueryAssets {
	text: string;
	assets: string[];
}

export function processQuery(editorRef: Editor): QueryAssets {
	const query: QueryAssets = { text: '', assets: [] };
	const view = editorRef.view;
	if (!view) {
		throw new Error('No view found');
	}

	const stateJson = view.state.toJSON();

	function processNodeJson(node: any) {
		if (node.type === 'text' && node.text) {
			query.text += ' ' + node.text + ' ';
		} else if (node.type && node.type !== 'doc' && node.type !== 'paragraph') {
			if (node.type.includes('-') || node.type.length > 10) {
				query.assets.push(node.attrs?.id ?? '');
				query.text += ' ' + node.attrs?.name + ' ';
			}
		}

		if (node.content && Array.isArray(node.content)) {
			node.content.forEach(processNodeJson);
		}
	}

	if (stateJson.doc?.content) {
		stateJson.doc.content.forEach(processNodeJson);
	}

	return query;
}

export async function clearQuery(editorRef: Editor) {
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		tr.delete(0, state.doc.content.size);
		dispatch?.(tr);
	});
}

function isExtensionNodeType(typeName: string): boolean {
	return (
		typeName !== 'doc' &&
		typeName !== 'paragraph' &&
		typeName !== 'text' &&
		(typeName.includes('-') || typeName.length > 10)
	);
}

export async function clearExtensionNodes(editorRef: Editor) {
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const nodesToDelete: { from: number; to: number }[] = [];

		state.doc.descendants((node, pos) => {
			if (isExtensionNodeType(node.type.name)) {
				nodesToDelete.push({
					from: pos,
					to: pos + node.nodeSize,
				});
			}
			return true;
		});

		// Delete in reverse order to maintain position accuracy
		nodesToDelete.reverse().forEach(({ from, to }) => {
			tr.delete(from, to);
		});

		if (nodesToDelete.length > 0) {
			dispatch?.(tr);
		}
	});
}
