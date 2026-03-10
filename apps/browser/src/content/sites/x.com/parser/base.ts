import type { NativeTwitterTweet } from '../../../../shared/content/bindings';
import type { ParseResult } from './types';

export abstract class BasePageParser {
	abstract parse(doc: Document): ParseResult;

	protected extractTweets(doc: Document): NativeTwitterTweet[] {
		const tweets: NativeTwitterTweet[] = [];
		const tweetElements = doc.querySelectorAll('[data-testid="tweetText"]');

		tweetElements.forEach((tweetElement) => {
			const spanElement = tweetElement.querySelector('span');
			if (spanElement && spanElement.textContent) {
				tweets.push({
					text: spanElement.textContent.trim(),
					timestamp: null,
					author: null,
				});
			}
		});

		return tweets;
	}
}
