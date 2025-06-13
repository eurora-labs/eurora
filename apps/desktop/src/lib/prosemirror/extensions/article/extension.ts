import { default as Article, articleAttrs, articleSchema } from './Article.svelte';
import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index';
import type { Component } from 'svelte';

export const ID = '309f0906-d48c-4439-9751-7bcf915cdfc5';

export function articleExtension(): SveltePMExtension {
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: articleAttrs,
				schema: articleSchema,
				// component: Article,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, Article as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
