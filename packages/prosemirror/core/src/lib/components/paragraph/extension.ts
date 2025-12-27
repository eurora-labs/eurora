import {
	default as Paragraph,
	paragraphAttrs,
	paragraphSchema,
} from '$lib/components/paragraph/Paragraph.svelte';
import type { SveltePMExtension } from '@eurora/prosemirror-core/index';

export function paragraphExtension() {
	return {
		name: 'paragraph' as const,
		svelteNodes: {
			paragraph: {
				attrs: paragraphAttrs,
				schema: paragraphSchema,
				component: Paragraph as any,
			},
		},
	} satisfies SveltePMExtension;
}
