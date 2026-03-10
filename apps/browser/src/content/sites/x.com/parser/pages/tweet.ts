import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class TweetPageParser extends BasePageParser {
	parse(doc: Document): ParseResult {
		const tweets = this.extractTweets(doc);
		const mainTweet = tweets[0] ?? null;
		const replies = tweets.slice(1);

		return {
			page: 'tweet',
			data: { tweet: mainTweet, replies },
		};
	}
}
