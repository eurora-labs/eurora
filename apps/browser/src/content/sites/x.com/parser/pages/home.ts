import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class HomePageParser extends BasePageParser {
	parse(doc: Document): ParseResult {
		const tweets = this.extractTweets(doc);

		return {
			page: 'home',
			data: { tweets },
		};
	}
}
