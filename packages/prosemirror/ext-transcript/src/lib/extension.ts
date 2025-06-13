import { default as Transcript, transcriptAttrs, transcriptSchema } from './Transcript.svelte';
import { Editor, type SveltePMExtension, SvelteNodeView } from '@eurora/prosemirror-core/index.js';
import type { Component } from 'svelte';

export const ID = 'D8215655-A880-4B0F-8EFA-0B6B447F8AF3';

export function transcriptExtension() {
	return {
		name: ID,
		svelteNodes: {
			transcript: {
				attrs: transcriptAttrs,
				schema: transcriptSchema,
				// component: Transcript,
				nodeView: (editor: Editor) =>
					SvelteNodeView.fromComponent(editor, Transcript as unknown as Component),
			},
		},
	} satisfies SveltePMExtension;
}
