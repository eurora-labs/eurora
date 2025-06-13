import type { SveltePMExtension } from '$lib/typings/extension.js';

export interface Query {
	text: string;
	extensions: SveltePMExtension[];
}
