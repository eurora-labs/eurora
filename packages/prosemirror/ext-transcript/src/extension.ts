import { default as Transcript, transcriptAttrs, transcriptSchema } from './Transcript.svelte';
import type { Component } from 'svelte';

import { SveltePMExtension } from '@eurora/prosemirror-core';
import { SvelteNodeView } from '@eurora/prosemirror-core';
export const ID = 'D8215655-A880-4B0F-8EFA-0B6B447F8AF3';

export function transcriptExtension() {
	return {
		name: ID,
		svelteNodes: {
			transcript: {
				attrs: transcriptAttrs,
				schema: transcriptSchema,
				// component: Transcript,
				nodeView: (editor: any) =>
					SvelteNodeView.fromComponent(editor, Transcript as unknown as Component)
			}
		}
	} satisfies SveltePMExtension;
}
