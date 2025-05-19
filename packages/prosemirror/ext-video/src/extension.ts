import { default as Video, videoAttrs, videoSchema } from './Video.svelte';
import type { Component } from 'svelte';

import { SveltePMExtension } from '@eurora/prosemirror-core';
import { SvelteNodeView } from '@eurora/prosemirror-core';

export function videoExtension() {
	return {
		id: '9370B14D-B61C-4CE2-BDE7-B18684E8731A',
		name: 'video' as const,
		svelteNodes: {
			video: {
				attrs: videoAttrs,
				schema: videoSchema,
				// component: Video,
				nodeView: (editor: any) =>
					SvelteNodeView.fromComponent(editor, Video as unknown as Component)
			}
		}
	} satisfies SveltePMExtension;
}
