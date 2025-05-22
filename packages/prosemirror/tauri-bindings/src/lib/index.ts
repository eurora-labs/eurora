import { Editor } from '@eurora/prosemirror-core';
import { Fragment } from 'prosemirror-model';
function handleFragment(content: Fragment) {
	content.forEach((node) => {
		if (node.type === 'video') {
			console.log(node.attrs);
		}
	});
}

export function processQuery(editorRef: Editor): string {
	const view = editorRef.view;
	if (!view) {
		throw new Error('No view found');
	}
	const doc = view.state.doc;
	let query = '';
	doc.content.forEach((node) => {
		if (node.type == 'text') {
			query += node.text;
		}
		if (node.type.name === 'video') {
			console.log(node.attrs);
		}
	});
	return query;
}
