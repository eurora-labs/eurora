import { Editor } from '@eurora/prosemirror-core/index';
import type { ContextChip } from '$lib/bindings/bindings.js';

export function executeCommand(editorRef: Editor, command: ContextChip) {
	if (!editorRef) return;
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const { schema } = state;
		const nodes = schema.nodes;
		const position = Math.max(command.position ?? 1, 1);
		tr.insert(
			position,
			// command.position ?? 0,
			nodes[command.extension_id].createChecked(
				{ id: command.id, name: command.name, ...command.attrs },
				// schema.text(command.name ?? ' '),
			),
		);
		dispatch?.(tr);
	});
}
