import { SveltePMExtension } from './extension.js';

export interface QueryExtension {
	position: number;
	extension: SveltePMExtension;
}

export interface Query {
	text: string;
	extensions: QueryExtension[];
}
