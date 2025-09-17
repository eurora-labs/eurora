import { Editor } from '@eurora/prosemirror-core/index';
import type { ContextChip } from '$lib/bindings/bindings.js';

export function executeCommand(editorRef: Editor, command: ContextChip) {
	if (!editorRef) return;
	console.log('command', command);
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const { schema } = state;
		const nodes = schema.nodes;
		tr.insert(
			command.position ?? 0,
			nodes[command.extension_id].createChecked(
				{ id: command.id, name: command.name, ...command.attrs },
				// schema.text(command.name ?? ' '),
			),
		);
		dispatch?.(tr);
	});
}
