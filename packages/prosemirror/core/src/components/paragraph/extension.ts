import { default as Paragraph, paragraphAttrs, paragraphSchema } from './Paragraph.svelte';
import type { SveltePMExtension } from '@prosemirror-svelte/core';

export function paragraphExtension() {
	return {
		name: 'paragraph' as const,
		svelteNodes: {
			paragraph: {
				attrs: paragraphAttrs,
				schema: paragraphSchema,
				component: Paragraph as any
			}
		}
	} satisfies SveltePMExtension;
}
