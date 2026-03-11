import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class NotificationsPageParser extends BasePageParser {
	async parse(doc: Document): Promise<ParseResult> {
		const tweets = await this.extractTweets(doc);

		return {
			page: 'notifications',
			data: { tweets },
		};
	}
}
