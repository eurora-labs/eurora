import { BasePageParser } from '../base';
import type { ParseResult } from '../types';

export class TweetPageParser extends BasePageParser {
	async parse(doc: Document): Promise<ParseResult> {
		const tweets = await this.extractTweets(doc);
		const mainTweet = tweets[0] ?? null;
		const replies = tweets.slice(1);

		return {
			page: 'tweet',
			data: { tweet: mainTweet, replies },
		};
	}
}
