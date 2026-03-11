import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class UnsupportedPageParser extends BasePageParser {
	async parse(_doc: Document): Promise<ParseResult> {
		return {
			page: 'unsupported',
			data: { url: window.location.href },
		};
	}
}
