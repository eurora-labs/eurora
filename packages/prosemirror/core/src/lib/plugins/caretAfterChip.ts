import { Plugin, PluginKey } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

const caretAfterChipKey = new PluginKey('caretAfterChip');

// export const caretAfterChip = new Plugin({
// 	key: caretAfterChipKey,

// 	props: {
// 		decorations(state) {
// 			const decos: Decoration[] = [];

// 			state.doc.descendants((node, pos) => {
// 				if (!node.isTextblock || node.childCount === 0) return;

// 				// Check if the last child is an inline atom (like our article chip)
// 				const lastChild = node.child(node.childCount - 1);
// 				if (lastChild.isInline && lastChild.isAtom) {
// 					// Calculate position after the last child
// 					const afterPos = pos + node.nodeSize - 1; // Just before the closing tag

// 					// Add a zero-width space widget to allow cursor positioning after the chip
// 					decos.push(
// 						Decoration.widget(
// 							afterPos,
// 							() => {
// 								const span = document.createElement('span');
// 								span.setAttribute('data-caret-after-chip', 'true');
// 								span.textContent = '\u200B'; // Zero-width space
// 								return span;
// 							},
// 							{
// 								side: 1, // Place after the node
// 								key: `caret-after-chip-${afterPos}`,
// 							},
// 						),
// 					);
// 				}
// 			});

// 			return DecorationSet.create(state.doc, decos);
// 		},
// 	},
// });
export const caretAfterChip = new Plugin({
	props: {
		decorations(state) {
			const decos: Decoration[] = [];
			console.log(state.doc.content.content[0].content.content.length);
			state.doc.descendants((node, pos) => {
				if (!node.isTextblock || node.childCount === 0) return;
				const last = node.child(node.childCount - 1);
				if (
					state.doc.content.content[0].content.content.length <= 1 &&
					last.type.name === 'text'
				)
					return;
				if (last.isInline && last.isAtom) {
					// if (last.type.name !== 'text') {
					const endPos = pos + node.nodeSize - 1; // block end
					decos.push(
						Decoration.widget(endPos, () => document.createTextNode('\u200B'), {
							side: 0,
						}),
					);
				}
			});
			return DecorationSet.create(state.doc, decos);
		},
	},
});
