import { default as Video, videoAttrs, videoSchema } from './Video.svelte';
import type { Component } from 'svelte';

import { SveltePMExtension } from '@eurora/prosemirror-core';
import { SvelteNodeView } from '@eurora/prosemirror-core';

export function videoExtension() {
	return {
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
