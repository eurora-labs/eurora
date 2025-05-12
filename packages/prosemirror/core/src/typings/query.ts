import { SveltePMExtension } from './extension.js';

export interface Query {
	text: string;
	extensions: SveltePMExtension[];
}
