import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class HomePageParser extends BasePageParser {
	async parse(doc: Document): Promise<ParseResult> {
		const tweets = await this.extractTweets(doc);

		return {
			page: 'home',
			data: { tweets },
		};
	}
}
