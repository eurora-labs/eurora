import type { ParseResult } from './types';
import type { NativeTwitterTweet } from '../../../../shared/content/bindings';

async function imageToBase64(url: string): Promise<string | null> {
	try {
		const resp = await fetch(url);
		const blob = await resp.blob();
		return await new Promise<string>((resolve, reject) => {
			const reader = new FileReader();
			reader.onloadend = () => resolve(reader.result as string);
			reader.onerror = reject;
			reader.readAsDataURL(blob);
		});
	} catch {
		return null;
	}
}

export abstract class BasePageParser {
	abstract parse(doc: Document): Promise<ParseResult>;

	protected async extractTweets(doc: Document): Promise<NativeTwitterTweet[]> {
		const tweets: NativeTwitterTweet[] = [];
		const tweetArticles = doc.querySelectorAll('article[data-testid="tweet"]');

		for (const article of Array.from(tweetArticles)) {
			const tweetTextEl = article.querySelector('[data-testid="tweetText"]');
			const spanElement = tweetTextEl?.querySelector('span');
			if (!spanElement || !spanElement.textContent) continue;

			const images: string[] = [];
			const imgElements = Array.from(
				article.querySelectorAll('[data-testid="tweetPhoto"] img'),
			);
			for (const img of imgElements) {
				const src = (img as HTMLImageElement).src;
				if (!src) continue;
				const base64 = await imageToBase64(src);
				if (base64) images.push(base64);
			}

			tweets.push({
				text: spanElement.textContent.trim(),
				timestamp: null,
				author: null,
				images,
			});
		}

		return tweets;
	}
}
