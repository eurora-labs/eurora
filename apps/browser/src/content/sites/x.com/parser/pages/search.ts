import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class SearchPageParser extends BasePageParser {
	parse(doc: Document): ParseResult {
		const params = new URLSearchParams(window.location.search);
		const query = params.get('q') ?? '';
		const tweets = this.extractTweets(doc);

		return {
			page: 'search',
			data: { query, tweets },
		};
	}
}
