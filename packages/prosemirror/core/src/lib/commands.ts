/* eslint-disable */

import { wrapIn } from 'prosemirror-commands';
import { Fragment, Mark, NodeRange, NodeType, Node as PMNode, Slice } from 'prosemirror-model';
import { TextSelection } from 'prosemirror-state';
import { findWrapping, liftTarget } from 'prosemirror-transform';
import type { Cmd } from './typings/index.js';

export const commands = {
	backspace:
		(times = 1): Cmd =>
		(state, dispatch) => {
			const { selection, tr } = state;
			const { from, empty } = selection;
			if (empty) {
				tr.delete(from - times, from);
			} else {
				tr.deleteSelection();
				if (times > 1) {
					tr.delete(from - times, from);
				}
			}

			dispatch && dispatch(tr);
			return true;
		},
	deleteBetween:
		(start?: number, end?: number): Cmd =>
		(state, dispatch) => {
			const tr = state.tr;
			const { from, to } = state.selection;
			let delStart: number, delEnd: number;
			if (start !== undefined) {
				delStart = start;
			} else if (from === to) {
				delStart = 0;
			} else {
				delStart = from;
			}
			if (end !== undefined) {
				delEnd = end;
			} else if (from === to) {
				delEnd = state.doc.nodeSize - 2;
			} else {
				delEnd = to;
			}
			tr.delete(delStart, delEnd);
			dispatch && dispatch(tr);
			return true;
		},
	focus: (): Cmd => (state, dispatch, view) => {
		if (view.hasFocus()) {
			return false;
		}
		view.focus();
		dispatch && dispatch(state.tr.scrollIntoView());
		return true;
	},
	insertMark:
		(mark: Mark, start?: number, end?: number): Cmd =>
		(state, dispatch) => {
			const tr = state.tr;
			const { from, to } = state.selection;
			tr.addMark(start ?? from, end ?? to, mark);
			dispatch && dispatch(tr);
			return true;
		},
	insertNode:
		(node: PMNode, pos?: number): Cmd =>
		(state, dispatch) => {
			const { selection, tr } = state;
			tr.insert(pos ?? selection.head, node);
			dispatch && dispatch(tr);
			return true;
		},
	insertText:
		(text: string, pos?: number): Cmd =>
		(state, dispatch) => {
			const tr = state.tr;
			tr.insertText(text, pos);
			dispatch && dispatch(tr);
			return true;
		},
	liftNode:
		(pos: number): Cmd =>
		(state, dispatch) => {
			const startPos = state.doc.resolve(pos);
			const node = state.doc.nodeAt(pos);
			if (!node) {
				return false;
			}
			const range = startPos.blockRange(state.doc.resolve(startPos.pos + node.nodeSize));
			const targetDepth = range ? Number(liftTarget(range)) : NaN;
			if (range && !Number.isNaN(targetDepth)) {
				dispatch && dispatch(state.tr.lift(range, targetDepth));
				return true;
			}
			return false;
		},
	moveCursor:
		(moved: 'start' | 'end' | number): Cmd =>
		(state, dispatch, view) => {
			const { from } = state.selection;
			if (moved === 'start') {
				dispatch && dispatch(state.tr.setSelection(TextSelection.atStart(state.doc)));
			} else if (moved === 'end') {
				dispatch && dispatch(state.tr.setSelection(TextSelection.atEnd(state.doc)));
			} else {
				return commands.selectText(from + moved)(state, dispatch, view);
			}
			return true;
		},
	paste:
		(slice: Slice, start?: number, end?: number): Cmd =>
		(state, dispatch) => {
			const {
				selection: { from, to },
				tr,
			} = state;
			dispatch &&
				dispatch(
					tr
						.replace(start ?? from, end ?? to, slice)
						.setMeta('paste', true)
						.setMeta('uiEvent', 'paste'),
				);
			return true;
		},
	replace:
		(content: Fragment | PMNode | PMNode[], start?: number, end?: number): Cmd =>
		(state, dispatch) => {
			const { tr } = state;
			tr.replaceWith(start ?? 0, end ?? state.doc.nodeSize - 2, content);
			dispatch && dispatch(tr);
			return true;
		},
	selectText:
		(start: number, end?: number): Cmd =>
		(state, dispatch) => {
			const { tr } = state;
			tr.setSelection(TextSelection.create(state.doc, start, end));
			dispatch && dispatch(tr);
			return true;
		},
	setNodeMarkup:
		(pos: number, attrs?: Record<string, any>): Cmd =>
		(state, dispatch) => {
			dispatch && dispatch(state.tr.setNodeMarkup(pos, undefined, attrs));
			return true;
		},
	wrapIn:
		(nodeType: NodeType, attrs?: { [key: string]: any }): Cmd =>
		(state, dispatch, view) => {
			return wrapIn(nodeType, attrs)(state, dispatch, view);
		},
	wrapInInline:
		(nodeType: NodeType, attrs?: { [key: string]: any }): Cmd =>
		(state, dispatch) => {
			const range = new NodeRange(
					state.selection.$from,
					state.selection.$to,
					state.selection.$from.depth,
				),
				wrapping = findWrapping(range, nodeType, attrs);
			if (wrapping) {
				dispatch && dispatch(state.tr.wrap(range, wrapping));
			}
			return !!wrapping;
		},
};

export type Commands = typeof commands;
