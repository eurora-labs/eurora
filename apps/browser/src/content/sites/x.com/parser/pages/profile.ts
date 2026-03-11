import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class ProfilePageParser extends BasePageParser {
	async parse(doc: Document): Promise<ParseResult> {
		const username = window.location.pathname.split('/')[1] ?? '';
		const tweets = await this.extractTweets(doc);

		return {
			page: 'profile',
			data: { username, tweets },
		};
	}
}
