import type { Editor } from '@eurora/prosemirror-core';
import { v7 as uuidv7 } from 'uuid';
export interface PMCommand {
	extension_id: string;
	position?: number;
	text?: string;
	name?: string;
	attrs?: Record<string, any>;
}
export function executeCommand(editorRef: Editor, command: PMCommand) {
	if (!editorRef) return;
	console.log('command', command);
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const { schema } = state;
		const nodes = schema.nodes;
		const id = uuidv7();
		tr.insert(
			command.position ?? 0,
			nodes[command.extension_id].createChecked(
				{ id, name: command.name, ...command.attrs },
				schema.text(command.text ?? ' ')
			)
		);
		dispatch?.(tr);
	});
}
