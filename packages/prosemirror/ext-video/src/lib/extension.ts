import { default as Video, videoAttrs, videoSchema } from './Video.svelte';
import type { Component } from 'svelte';

import { type SveltePMExtension } from '@eurora/prosemirror-core';
import { SvelteNodeView } from '@eurora/prosemirror-core';
export const ID = '9370B14D-B61C-4CE2-BDE7-B18684E8731A';

export function videoExtension(): SveltePMExtension {
	console.log('videoAttrs', videoAttrs);
	console.log('videoSchema', videoSchema);
	console.log('video', Video);
	return {
		name: ID,
		svelteNodes: {
			[ID]: {
				attrs: videoAttrs,
				schema: videoSchema,
				// component: Video,
				nodeView: (editor: any) =>
					SvelteNodeView.fromComponent(editor, Video as unknown as Component)
			}
		}
	} satisfies SveltePMExtension;
}
