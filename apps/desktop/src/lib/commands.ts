import type { Editor } from '@eurora/prosemirror-core';
import { v7 as uuidv7 } from 'uuid';
export interface PMCommand {
	name: string;
	position?: number;
	text?: string;
	attrs?: Record<string, any>;
}
export function executeCommand(editorRef: Editor, command: PMCommand) {
	if (!editorRef) return;
	editorRef.cmd((state, dispatch) => {
		const tr = state.tr;
		const { schema } = state;
		const nodes = schema.nodes;
		const id = uuidv7();
		tr.insert(
			command.position ?? 0,
			nodes[command.name].createChecked({ id, ...command.attrs }, schema.text(command.text ?? ''))
		);
		dispatch?.(tr);
	});
}
