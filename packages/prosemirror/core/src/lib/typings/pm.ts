import { EditorState, Transaction } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';

export type Cmd = (
	state: EditorState,
	dispatch: ((tr: Transaction) => void) | undefined,
	view: EditorView,
) => boolean | undefined | void;

export interface Commands {
	[name: string]: (...args: any[]) => Cmd;
}

// eslint-disable-next-line
export interface PMDoc extends Record<string, any> {}

// eslint-disable-next-line
export interface DocJSON extends Record<string, any> {}
export interface EditorStateJSON {
	doc: DocJSON;
	selection?: { [key: string]: any };
}
