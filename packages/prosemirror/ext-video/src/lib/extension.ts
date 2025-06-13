import { default as Video, videoAttrs, videoSchema } from './Video.svelte';
import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index';
import type { Component } from 'svelte';

export const ID = '9370B14D-B61C-4CE2-BDE7-B18684E8731A';

export function videoExtension(): SveltePMExtension {
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: videoAttrs,
				schema: videoSchema,
				// component: Video,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, Video as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
