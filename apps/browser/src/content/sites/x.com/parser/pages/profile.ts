import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class ProfilePageParser extends BasePageParser {
	parse(doc: Document): ParseResult {
		const username = window.location.pathname.split('/')[1] ?? '';
		const tweets = this.extractTweets(doc);

		return {
			page: 'profile',
			data: { username, tweets },
		};
	}
}
