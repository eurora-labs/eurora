import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class UnsupportedPageParser extends BasePageParser {
	parse(_doc: Document): ParseResult {
		return {
			page: 'unsupported',
			data: { url: window.location.href },
		};
	}
}
