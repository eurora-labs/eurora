import { Editor } from '@eurora/prosemirror-core';
export function processQuery(editorRef: Editor): string {
	const view = editorRef.view;
	if (!view) {
		throw new Error('No view found');
	}
	const doc = view.state.doc;
	const query = '';
	doc.content.forEach((node) => {
		if (node.type.name === 'video') {
			console.log(node.attrs);
		}
	});
	return query;
}
