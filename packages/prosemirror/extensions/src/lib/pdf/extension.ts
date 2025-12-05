import { default as Pdf, pdfAttrs, pdfSchema } from './Pdf.svelte';
import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index';
import type { Component } from 'svelte';

export const ID = '59b26f84-d10a-11f0-a0a4-17b6bfaafdde';

export function pdfExtension(): SveltePMExtension {
	return {
		name: ID,

		svelteNodes: {
			[ID]: {
				attrs: pdfAttrs,
				schema: pdfSchema,
				// component: Pdf,,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, Pdf as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
