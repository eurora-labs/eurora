import { default as YoutubeVideo, videoAttrs, videoSchema } from './YoutubeVideo.svelte';
import type { Component } from 'svelte';

import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index';
export const ID = '7c7b59bb-d44d-431a-9f4d-64240172e092';

export function youtubeVideoExtension(): SveltePMExtension {
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: videoAttrs,
				schema: videoSchema,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, YoutubeVideo as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
