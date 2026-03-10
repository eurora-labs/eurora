import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class NotificationsPageParser extends BasePageParser {
	parse(doc: Document): ParseResult {
		const tweets = this.extractTweets(doc);

		return {
			page: 'notifications',
			data: { tweets },
		};
	}
}
