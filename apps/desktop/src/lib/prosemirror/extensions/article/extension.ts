import { default as Article, articleAttrs, articleSchema } from './Article.svelte';
import type { Component } from 'svelte';

import { type SveltePMExtension } from '@eurora/prosemirror-core';
import { SvelteNodeView } from '@eurora/prosemirror-core';
export const ID = '309f0906-d48c-4439-9751-7bcf915cdfc5';

export function articleExtension(): SveltePMExtension {
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: articleAttrs,
				schema: articleSchema,
				// component: Article,
				nodeView: (editor: any) =>
					SvelteNodeView.fromComponent(editor, Article as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
