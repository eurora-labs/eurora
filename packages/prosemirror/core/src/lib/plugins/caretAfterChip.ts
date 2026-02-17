import { Plugin } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

export const caretAfterChip = new Plugin({
	props: {
		decorations(state) {
			const decos: Decoration[] = [];
			state.doc.descendants((node, pos) => {
				if (!node.isTextblock || node.childCount === 0) return;
				const last = node.child(node.childCount - 1);
				if (
					state.doc.content.content[0].content.content.length <= 1 &&
					last.type.name === 'text'
				)
					return;
				if (last.isInline && last.isAtom) {
					const endPos = pos + node.nodeSize - 1;
					decos.push(
						Decoration.widget(endPos, () => document.createTextNode('\u200B'), {
							side: 1,
						}),
					);
				}
			});
			return DecorationSet.create(state.doc, decos);
		},
	},
});
