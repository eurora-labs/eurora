import { default as TwitterPost, tweetAttrs, tweetSchema } from './TwitterPost.svelte';
import type { Component } from 'svelte';

import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index';
export const ID = '2c434895-d32c-485f-8525-c4394863b83a';

export function twitterExtension(): SveltePMExtension {
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: tweetAttrs,
				schema: tweetSchema,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, TwitterPost as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
